#![feature(refcell_replace_swap)]
///
/// Implementation of a client that communicates with the relay server
/// this implememnataion is simplistic and used for POC and development and debugging of the
/// server
///
///
extern crate futures;
extern crate tokio_core;
extern crate relay_server_common;
extern crate dict;


use std::env;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::{thread, time};
use std::cell::RefCell;

use tokio_core::reactor::Core;
use tokio_core::net::TcpStream;
use tokio_core::io::Io;

use futures::{Stream, Sink, Future};
use futures::sync::mpsc;

use relay_server_common::{
    ClientToServerCodec,
    ClientMessage,
    ServerMessage,
    ServerResponse,
    RelayMessage,
    ProtocolIdentifier,
    PeerIdentifier
};

// unique to our eddsa client
extern crate multi_party_ed25519;
extern crate curv;

use curv::elliptic::curves::ed25519::*;
use multi_party_ed25519::protocols::aggsig::{
    test_com, verify, KeyPair, Signature, EphemeralKey
};

use relay_server_common::common::*;

use dict::{ Dict, DictIface };

// ClientSession holds session data
#[derive(Default, Debug, Clone)]
struct ProtocolSession {
    pub registered: bool,
    pub peer_id: RefCell<PeerIdentifier>,
    pub protocol_id: ProtocolIdentifier,
    pub capacity: u32,
    pub next_message: Option<ClientMessage>,
    pub bc_dests: Vec<ProtocolIdentifier>,
    pub step: u32,
    //pub  protocol_data: ProtocolData,
}


impl ProtocolSession {
    pub fn new(protocol_id:ProtocolIdentifier, capacity: u32) -> ProtocolSession {
        ProtocolSession {
            registered: false,
            peer_id: RefCell::new(0),
            protocol_id,
            capacity,
            next_message: None,
            bc_dests: (1..(capacity+1)).collect(),
            //protocol_data: ProtocolData::new(),
            step: 0,
        }
    }

    pub fn set_bc_dests(&mut self){
        let index = self.peer_id.clone().into_inner() - 1;
        self.bc_dests.remove(index as usize);
    }

    pub fn next_step(&mut self) {
        let step = self.clone().step + 1;
        self.step = step + 1;
    }
}


#[derive(Debug)]
pub enum ServerMessageType { // TODO this is somewhat duplicate
    Response,
    Abort,
    RelayMessage,
    Undefined,
}

pub fn resolve_msg_type(msg: ServerMessage) -> ServerMessageType{
    if msg.response.is_some(){
        return ServerMessageType::Response;
    }
    if msg.relay_message.is_some(){
        return ServerMessageType::RelayMessage;
    }
    if msg.abort.is_some(){
        return ServerMessageType::Abort;
    }
    return ServerMessageType::Undefined;
}

pub enum MessageProcessResult {
    Message,
    NoMessage,
    Abort
}

fn generate_pk_message_payload(pk: &String) -> String {
    return format!("{}{}{}", PK_MESSAGE_PREFIX, RELAY_MESSAGE_DELIMITER, pk)
}

enum RELAY_MESSAGE_TYPE {
    /// Types of expected relay messages
    /// for step 0 we expect PUBLIC_KEY_MESSAGE
    /// for step 1 we expect COMMITMENT
    /// for step 2 we expect R_MESSAGE
    /// for step 3 we expect SIGNATURE

    PUBLIC_KEY(String), //  Serialized key
    COMMITMENT(String), //  Commitment
    R_MESSAGE(String),  //  R_j of the peer
    SIGNATURE(String),  //  S_j
}


struct PeerData{
    /// data structure that holds the relevant data of the peers.
    /// In our case:
    /// pks: all the public keys of the peers
    /// TODO
    /// TODO
    pub pks: Vec<String>,
    pub commitments: Vec<String>,
    pub r_s: Vec<String>,
    pub sigs: Vec<String>,
}

impl PeerData {
    pub fn new() -> PeerData{
        PeerData {
            pks: Vec::new(),
            commitments: Vec::new(),
            r_s: Vec::new(),
            sigs: Vec::new(),
        }

    }
}



fn resolve_relay_message_type(msg: &RelayMessage) -> RELAY_MESSAGE_TYPE {
    let msg_payload = msg.message.clone();

    let split_msg:Vec<&str> = msg_payload.split(RELAY_MESSAGE_DELIMITER).collect();
    let msg_prefix = split_msg[0];
    let msg_payload = String::from( split_msg[1].clone());
    match msg_prefix {

        pk_prefix if pk_prefix == String::from(PK_MESSAGE_PREFIX)=> {
            return RELAY_MESSAGE_TYPE::PUBLIC_KEY(msg_payload);
        },
        cmtmnt if cmtmnt == String::from(COMMITMENT_MESSAGE_PREFIX) => {
            return RELAY_MESSAGE_TYPE::COMMITMENT(msg_payload);
        },
        r if r == String::from(R_KEY_MESSAGE_PREFIX ) => {
            return RELAY_MESSAGE_TYPE::R_MESSAGE(msg_payload);

        },
        sig if sig == String::from(SIGNATURE_MESSAGE_PREFIX)=> {
            return RELAY_MESSAGE_TYPE::SIGNATURE(msg_payload);
        },
        _ => panic!("Unknown relay message prefix")
    }
}

fn main() {
    // message for signing
    let message: [u8; 4] = [79,77,69,82];


    let PROTOCOL_IDENTIFIER_ARG = 1;
    let PROTOCOL_CAPACITY_ARG = 2 as ProtocolIdentifier;

    let mut args = env::args().skip(1).collect::<Vec<_>>();
    // Parse what address we're going to co nnect to
    let addr = args.first().unwrap_or_else(|| {
        panic!("this program requires at least one argument")
    });

    let addr = addr.parse::<SocketAddr>().unwrap();

    // Create the event loop and initiate the connection to the remote server
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let _tcp = TcpStream::connect(&addr, &handle);


    let mut session = ProtocolSession::new(PROTOCOL_IDENTIFIER_ARG, PROTOCOL_CAPACITY_ARG);
    let mut peer_data = PeerData::new();

    let client = _tcp.and_then(|stream| {
        println!("sending register message");

        let framed_stream = stream.framed(ClientToServerCodec::new());


        // prepare register message
        let mut msg = ClientMessage::new();
        let register_msg = msg.register(session.protocol_id.clone(), session.capacity.clone());

        // let mut session = session.clone();
        // send register message to server
	    let send_ = framed_stream.send(msg);
        send_.and_then(|stream| {
            let (tx, rx) = stream.split();
            let client = rx.and_then(|msg| {
                println!("Got message from server: {:?}", msg);
                let msg_type = resolve_msg_type(msg.clone());
                match msg_type {
                    ServerMessageType::Response =>{
                        let server_response = msg.response.unwrap();
                        match server_response {
                            ServerResponse::Register(peer_id) => {
                                println!("Peer identifier: {}",peer_id);
                                // Set the session parameters
                                session.peer_id.replace(peer_id);
                                session.set_bc_dests();
                                session.next_step();

                                //after register, generate signing key
                                let key = KeyPair::create();
                                let pk/*:Ed25519Point */= key.public_key;
                                let message =  serde_json::to_string(&pk).expect("Failed in serialization");key.public_key;

//                                    let (ephemeral_key, sign_first_message, sign_second_message) =
//                                        Signature::create_ephemeral_key_and_commit(&key, &message);
//
//                                    let commitment = &sign_first_message.commitment.clone();
//                                    println!("sending commitment");

                                // create relay message
                                let mut client_message= ClientMessage::new();
                                let mut relay_message = RelayMessage::new(peer_id, session.protocol_id.clone());
                                let mut to: Vec<u32> = session.bc_dests.clone();

                                // wait a little so we can spawn the second client
                                let wait_time = time::Duration::from_millis(5000);
                                thread::sleep(wait_time);

                                relay_message.set_message_params(0, to, generate_pk_message_payload(&message));
                                client_message.relay_message = Some(relay_message.clone());
                                return Ok(client_message);
                            },
                            ServerResponse::ErrorResponse(err_msg) => {
                                println!("got error response");
                                return Ok(ClientMessage::new());
                            },
                            ServerResponse::GeneralResponse(msg) => {
                                unimplemented!()
                            },
                            ServerResponse::NoResponse => {
                              unimplemented!()
                            },
                            _ => panic!("failed to register")
                        }
                    }
                    ServerMessageType::RelayMessage => {
                        println!("Got new relay message");
                        println!("{:?}", msg);
                        // parse relay message
                        // (if we got here this means we are registered and
                        // the client sent the private key)

                        // so at the first step we are expecting the pks from all other peers
                        let relay_msg = msg.relay_message.unwrap();
                        let msg_type = resolve_relay_message_type(&relay_msg);
                        match msg_type {
                            // for each type
                            // check if received data from all peers,
                            // if so do the next step,
                            // if not send empty message (means we are still waiting)
                            RELAY_MESSAGE_TYPE::PUBLIC_KEY(pk) => {
                                println!("Got public key: {:}", pk);
                                peer_data.pks.push(pk);
                            },
                            RELAY_MESSAGE_TYPE::COMMITMENT(t) => {
                                unimplemented!()
                            },
                            RELAY_MESSAGE_TYPE::R_MESSAGE(r) => {
                                unimplemented!()
                            },
                            RELAY_MESSAGE_TYPE::SIGNATURE(s) => {
                                unimplemented!()
                            }
                            _ => panic!("Unknown relay message type")
                        }
                        Ok(ClientMessage::new())
                    },
                    ServerMessageType::Abort => {
                        println!("Got abort message");
                        //Ok(MessageProcessResult::NoMessage)
                        Ok(ClientMessage::new())
                    },
                    ServerMessageType::Undefined => {
                        Ok(ClientMessage::new())
                        //panic!("Got undefined message: {:?}",msg);
                    }
                }
            }).forward(tx);
            client
        })
    })
        .map_err(|err| {
            // All tasks must have an `Error` type of `()`. This forces error
            // handling and helps avoid silencing failures.
            //
            // In our example, we are only going to log the error to STDOUT.
            println!("connection error = {:?}", err);
        });


    core.run(client);//.unwrap();

}
