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
use std::vec::Vec;

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
    PeerIdentifier,
    MessagePayload,
};

// unique to our eddsa client
extern crate multi_party_ed25519;
extern crate curv;

use curv::elliptic::curves::ed25519::*;
use multi_party_ed25519::protocols::aggsig::{
    test_com, verify, KeyPair, Signature, EphemeralKey, KeyAgg, SignFirstMsg, SignSecondMsg
};
//use multi_party_ed25519::

use relay_server_common::common::*;


use dict::{ Dict, DictIface };
use std::collections::HashMap;

struct EddsaPeer{
    pub peer_id: RefCell<PeerIdentifier>,
    pub client_key: Option<KeyPair>,
    pub pks: HashMap<PeerIdentifier, Ed25519Point>,
    pub commitments: HashMap<PeerIdentifier, String>,
    pub r_s: HashMap<PeerIdentifier, String>,
    pub sigs: HashMap<PeerIdentifier, String>,
    pub capacity: u32,
    pub message: &'static[u8],
    pub agg_key: Option<KeyAgg>,
    pub R_tot: Option<GE>,
    //pub current_step: u32,
}

//commitment is of type signFirstMessage
// R is of type signSecondMessage
impl EddsaPeer{
    fn add_pk(&mut self, peer_id: PeerIdentifier, pk: Ed25519Point){
        self.pks.insert(peer_id, pk);
    }
    fn add_commitment(&mut self, peer_id: PeerIdentifier, commitment: String){self.commitments.insert(peer_id, commitment);/*TODO*/}
    fn add_r(&mut self, peer_id: PeerIdentifier, r:String){
        //let v = (r,blind_factor);
        self.r_s.insert(peer_id, r);
    }
    fn add_sig(&mut self, peer_id: PeerIdentifier, sig: String){
        self.sigs.insert(peer_id, sig);
    }
    fn compute_r_tot(&mut self) {
        let mut Ri:Vec<GE> = Vec::new();
        for (peer_id, r) in self.r_s {
            let r_sloce:&str = &r[..];
            let _r:SignSecondMsg = serde_json::from_str(r_slice).unwrap_or_else(panic!("serialization error"));
            Ri.push(_r.R.clone());
        }
        let r_tot= Signature::get_R_tot(Ri);
        self.R_tot = Some(r_tot);
    }
    fn aggregate_pks(&mut self) {
        let mut pks = Vec::with_capacity(self.capacity as usize);
        for (peer, pk) in self.pks {
            pks[(peer - 1) as usize] = pk;
        }
        let peer_id = self.peer_id.clone().into_inner();
        let index = (peer_id - 1) as usize;
        let agg_key= KeyPair::key_aggregation_n(&pks, &index);
        self.agg_key = Some(agg_key);
    }
    fn validate_commitments(&mut self) -> bool{
        // iterate over all peer Rs
        for (peer_id, r) in self.r_s {
            // convert the json_string to a construct
            let _r:R = serde_json::from_str(&r).unwrap();

            // get the corresponding commitment
            let k = self.peer_id.clone().into_inner();
            let cmtmnt = self.commitments.get(&k)
                .expect("peer didn't send commitment");
            let commitment = serde_json::from_str(cmtmnt).unwrap();
            // if we couldn't validate the commitment - failure
            if !test_com(
                &_r.R,
                &_r.blind_factor,
                commitment.unwrap_or_else(panic!("couldn't parse commitment json"))
            ) {
                return false;
            }
        }
        true
    }
}

impl EddsaPeer {
    /// data updaters for each step
    pub fn update_data_step_0(&mut self, from: PeerIdentifier, payload: MessagePayload) {
        let payload_type = self.resolve_payload_type(&payload);
        match payload_type {
            MessagePayloadType::PUBLIC_KEY(pk) => {
                let s_slice: &str = &pk[..];  // take a full slice of the string
                let _pk = serde_json::from_str(s_slice);
                match _pk {
                    Ok(_pk) => self.add_pk(from, _pk),
                    Err(e) => panic!("Could not serialize public key")
                }
            },
            _ => panic!("expected public key message")
        }
    }

    pub fn update_data_step_1(&mut self, from: PeerIdentifier, payload: MessagePayload) {
        let payload_type = self.resolve_payload_type(&payload);
        match payload_type {
            MessagePayloadType::COMMITMENT(t) => {
                self.add_commitment(from, t);
            },
            _ => panic!("expected commitment message")
        }
    }

    pub fn update_data_step_2(&mut self, from: PeerIdentifier, payload: MessagePayload) {
        let payload_type = self.resolve_payload_type(&payload);
        match payload_type {
            MessagePayloadType::R_MESSAGE(r) => {
                self.add_r(from, r);
            },
            _ => panic!("expected R message")
        }
    }

    pub fn update_data_step_3(&mut self, from: PeerIdentifier, payload: MessagePayload) {
        let payload_type = self.resolve_payload_type(&payload);
        match payload_type {
            MessagePayloadType::SIGNATURE(s) => {
                self.add_sig(from, s);
            },
            _ => panic!("expected signature message")
        }
    }
}

impl EddsaPeer {
    fn is_step_done(&mut self) -> bool {
        match self.current_step {
            0 => return self.is_done_step_0(),//from, payload), // in step 0 we move immediately to step 1
            1 => return self.is_done_step_1(),//from, payload),
            2 => return self.is_done_step_2(),//from, payload),
            3 => return self.is_done_step_3(),//from, payload),
            _ => panic!("Unsupported step")
        }
    }
    pub fn is_done_step_0(&self) -> bool {
        self.pks.len() == self.capacity as usize
    }
    pub fn is_done_step_1(&self) -> bool {
        self.commitments.len() == self.capacity as usize
    }
    pub fn is_done_step_2(&self) -> bool {
        self.r_s.len() == self.capacity as usize
    }
    pub fn is_done_step_3(&self) -> bool {
        self.sigs.len() == self.capacity as usize
    }
    /// Check if peer should finalize the session
    pub fn should_finalize(&mut self)->bool{
        self.is_done()
    }
}
impl EddsaPeer{
    /// steps - in each step the client does a calculation on its
    /// data, and updates the data holder with the new data
    pub fn step_1(&mut self) -> MessagePayload{
        // each peer computes its commitment to the ephemeral key
        // (this implicitly means each party also calculates ephemeral key
        // on this step)
        // round 1: send commitments to ephemeral public keys
        let k = self.client_key.unwrap_or_else(panic!("No client key"));
        let (ephemeral_key, sign_first_message, sign_second_message) =
            Signature::create_ephemeral_key_and_commit(&k, &self.message);

        //let commitment = sign_first_message.commitment;
        // save the commitment
        match serde_json::to_string(&sign_first_message){
            Ok(json_string) =>{
                self.add_commitment(self.peer_id.clone().into_inner(), json_string.clone());
                let r = serde_json::to_string(&sign_second_message).expect("couldn't create R");
                //let blind_factor = serde_json::to_string(&sign_second_message.blind_factor).expect("Couldn't serialize blind factor");
                self.add_r(self.peer_id.clone().into_inner(), r);
                return generate_commitment_message_payload((&json_string));
            } ,
            Err(e) => panic!("Couldn't serialize commitment")
        }
    }
    pub fn step_2(&mut self) -> MessagePayload{
        /// step 2 - return the clients R. No extra calculations
        let peer_id = self.peer_id.clone().into_inner();
        let r = self.r_s.get(&peer_id).unwrap_or_else(panic!("Didn't compute R"));
        return generate_R_message_payload(&r);

    }
    /// step 3 - after validating all commitments:
    /// 1. compute APK
    /// 2. compute R' = sum(Ri)
    /// 3. sign message
    /// 4. generate (and return) signature message payload
    pub fn step_3(&mut self) -> MessagePayload{

        if !self.validate_commitments() {
            // commitments sent by others are not valid. exit
            panic!("Commitments not valid!")
        }
        self.aggregate_pks();
        self.compute_r_tot();
        let R_tot = self.R_tot.unwrap_or_else(panic!("Didn't compute R_tot!"));
        let apk = self.agg_key.unwrap_or_else(panic!("Didn't compute apk!"));


        let k = Signature::k(&R_tot, &self.agg_key.apk, &self.message);
        let peer_id = self.peer_id.clone().into_inner();
        let r = self.r_s.get(&peer_id).unwrap_or_else(panic!("Client has No R ")).clone();
        let _r: SignSecondMsg = serde_json::from_str(&r);
        let key = &self.client_key.unwrap_or_else(panic!("No key"));
        // sign
        let s = Signature::partial_sign(&_r,&key,&k,&apk.hash,&R_tot);
        let sig_string = serde_json::to_string(&s).expect("failed to serialize signature");

        generate_signature_message_payload(&sig_string)
    }



}

impl EddsaPeer{
    fn resolve_payload_type(message: MessagePayload) -> MessagePayloadType  {
        let msg_payload = message.clone();

        let split_msg:Vec<&str> = msg_payload.split(RELAY_MESSAGE_DELIMITER).collect();
        let msg_prefix = split_msg[0];
        let msg_payload = String::from( split_msg[1].clone());
        match msg_prefix {

            pk_prefix if pk_prefix == String::from(PK_MESSAGE_PREFIX)=> {
                return MessagePayloadType ::PUBLIC_KEY(msg_payload);
            },
            cmtmnt if cmtmnt == String::from(COMMITMENT_MESSAGE_PREFIX) => {
                return MessagePayloadType ::COMMITMENT(msg_payload);
            },
            r if r == String::from(R_KEY_MESSAGE_PREFIX ) => {
                return MessagePayloadType::R_MESSAGE(msg_payload);

            },
            sig if sig == String::from(SIGNATURE_MESSAGE_PREFIX)=> {
                return MessagePayloadType ::SIGNATURE(msg_payload);
            },
            _ => panic!("Unknown relay message prefix")
        }
    }
}

impl Peer for EddsaPeer{
    fn new(capacity: u32, _message: &'static[u8]) -> EddsaPeer{
        EddsaPeer {
            client_key: None,
            pks: HashMap::new(),
            commitments: HashMap::new(),
            r_s: HashMap::new(),
            sigs: HashMap::new(),
            capacity,
            message: _message,
            peer_id: RefCell::new(0),
            agg_key: None,
            //current_step: 0,
            R_tot: None,
        }
    }

    fn zero_step(&mut self, peer_id:PeerIdentifier) -> Option<MessagePayload> {
        self.peer_id.replace(peer_id);
        self.client_key = Some(KeyPair::create());
        let pk/*:Ed25519Point */= self.client_key.unwrap().public_key.clone();
        self.add_pk(peer_id, pk);


        let pk/*:Ed25519Point */= self.client_key.public_key.unwarp_or_else(panic!("client key not created"));
        let pk_s = serde_json::to_string(&pk).expect("Failed in serialization");
        return Some(generate_pk_message_payload(&pk_s));
    }

    fn do_step(&mut self) ->Option<MessagePayload> {
        if self.is_step_done() {
            // do the next step
            self.current_step += 1;
            match self.current_step {
                1 => {return Some(self.data_holder.step_1())},
                2 => {return Some(self.data_holder.step_2())},
                3 => {return Some(self.data_holder.step_3())},
                _=>panic!("Unsupported step")
            }
        }
        None
    }

    fn update_data(&mut self, from: PeerIdentifier, payload: MessagePayload){
        // update data according to step
        match self.current_step {
            0 => self.update_data_step_0(from, payload),
            1 => self.update_data_step_1(from, payload),
            2 => self.update_data_step_2(from, payload),
            3 => self.update_data_step_3(from, payload),
            _=>panic!("Unsupported step")
        }

    }
    /// Does the final calculation of the protocol
    /// in this case:
    ///     collection all signatures
    ///     and verifying the message
    fn finalize(&mut self) -> Result<(),&'static str> {
        if !self.is_done(){
            return Err("not done")
        }
        // collect signatures
        let mut s: Vec<Signature> = Vec::new();
        for (peer_id, sig) in self.sigs {
            let signature = serde_json::from_str(&sig).expect("Could not serialize signature!");
            s.push(signature)
        }
        let signature = Signature::add_signature_parts(s);

        // verify message with signature
        let apk = self.agg_key.unwrap();
        if verify(&signature,&self.message, &apk.apk){
            Ok(())
        } else {
            Err("failed to verify message with aggregated signature")
        }

    }
    /// check that the protocol is done
/// and that this peer can finalize its calculations
    fn is_done(&mut self) -> bool {
        self.is_done_step_3()
    }

}
pub trait Peer {
    fn new(capacity: u32, _message: &'static[u8]) -> Self;
    fn zero_step(&mut self, peer_id:PeerIdentifier) -> Option<MessagePayload>;
    fn do_step(&mut self) ->Option<MessagePayload>;
    fn update_data(&mut self, from: PeerIdentifier, payload: MessagePayload);
    fn finalize(&mut self) -> Result<(),&'static str>;
    fn is_done(&mut self) -> bool;
}

struct ProtocolDataManager<T: Peer>{
    pub peer_id: RefCell<PeerIdentifier>,
    pub capacity: u32,
    pub current_step: u32,
    pub data_holder: T, // will be filled when initializing, and on each new step
    pub client_data: Option<MessagePayload>, // new data calculated by this peer at the beginning of a step (that needs to be sent to other peers)
    pub new_client_data: bool,
}

impl<T: Peer> ProtocolDataManager<T>{
    pub fn new(capacity: u32, message:&[u8]) -> ProtocolDataManager<T>
    where T: Peer{
        ProtocolDataManager {
            peer_id: RefCell::new(0),
            current_step: 0,
            capacity,
            data_holder: Peer::new(capacity, message),
            client_data: None,
            new_client_data: false,
            //message: message.clone(),
        }
    }

    /// set manager with the initial values that a local peer holds at the beginning of
    /// the protocol session
    /// return: first message
    pub fn initialize_data(&mut self, peer_id: PeerIdentifier) -> Option<MessagePayload>{
        self.peer_id.replace(peer_id);
        let zero_step_data = self.data_holder.zero_step(peer_id);
        self.client_data = zero_step_data;
        return self.client_data.clone();
    }



    /// Return the next data this peer needs
    /// to send to other peers
    pub fn get_next_message(&mut self, from: PeerIdentifier, payload: MessagePayload) -> MessagePayload{
        self.data_holder.update_data(from, payload);
        self.client_data = self.data_holder.do_step();
        let data = self.client_data;
        match data {
            Some(data) => {
                self.client_data = None;
                return data;
            },
            None => {
                let m = relay_server_common::common::EMPTY_MESSAGE_PAYLOAD.clone();
                return String::from(m);
            },
        }
    }
}


struct ProtocolSession<T> where T: Peer{
    pub registered: bool,
    pub protocol_id: ProtocolIdentifier,
    pub data_manager: ProtocolDataManager<T>,
    pub last_message: Option<ClientMessage>,
    pub bc_dests: Vec<ProtocolIdentifier>,
    pub timeout: u32,
}


impl<T: Peer> ProtocolSession<T> {
    pub fn new(protocol_id:ProtocolIdentifier, capacity: u32, message: &[u8]) -> ProtocolSession<T>
    where T: Peer {
        let data_m: ProtocolDataManager<T> = ProtocolDataManager::new(capacity, message);
        ProtocolSession {
            registered: false,
            protocol_id,
            last_message: None,
            bc_dests: (1..(capacity+1)).collect(),
            timeout: 5000,
            data_manager: data_m,
        }
    }

    fn set_bc_dests(&mut self){
        let index = self.data_manager.peer_id.clone().into_inner() - 1;
        self.bc_dests.remove(index as usize);
    }

    fn handle_relay_message(&mut self, msg: ServerMessage) -> Option<MessagePayload>{
        // parse relay message
        // (if we got here this means we are registered and
        // the client sent the private key)

        // so at the first step we are expecting the pks from all other peers
        let relay_msg = msg.relay_message.unwrap();
        let from = relay_msg.peer_number;
        let payload = relay_msg.message;
        let answer: MessagePayload = self.data_manager.get_next_message(from, payload);
        return Some(answer);
    }

    pub fn generate_client_answer(&mut self, msg: ServerMessage) -> Option<ClientMessage> {
        let msg_type = resolve_server_msg_type(msg.clone());
        match msg_type {
            ServerMessageType::Response =>{
                let next =self.handle_server_response(&msg);
                match next {
                    Ok(next_msg) => return Some(next_msg),
                    Err(e) => panic!("Error in handle_server_response"),
                }
            },
            ServerMessageType::RelayMessage => {
                println!("Got new relay message");
                println!("{:?}", msg);
                let next = self.handle_relay_message(msg.clone());
                match next {
                    Some(next_msg) => return Some(self.generate_relay_message(&next_msg)),
                    None => panic!("Error in handle_relay_message"),
                }
            },
            ServerMessageType::Abort => {
                println!("Got abort message");
                //Ok(MessageProcessResult::NoMessage)
                Some(ClientMessage::new())
            },
            ServerMessageType::Undefined => {
                Some(ClientMessage::new())
                //panic!("Got undefined message: {:?}",msg);
            }
        }
    }

    pub fn generate_register_message(&mut self) -> ClientMessage{
        let mut msg = ClientMessage::new();
        msg.register(self.protocol_id.clone(), self.data_manager.capacity.clone());
        msg
    }

    fn generate_relay_message(&self, payload: &MessagePayload) -> ClientMessage {
        let msg = ClientMessage::new();
        // create relay message
        let mut relay_message = RelayMessage::new(self.data_manager.peer_id.clone().into_inner(), self.protocol_id.clone());
        let mut to: Vec<u32> = self.bc_dests.clone();

        let mut client_message =  ClientMessage::new();
        let msg = String::from_str(payload);
        relay_message.set_message_params(to, msg);
        client_message.relay_message = Some(relay_message);
        client_message
    }

    fn handle_register_response(&mut self, peer_id: PeerIdentifier) ->Result<ClientMessage, ()>{
        println!("Peer identifier: {}",peer_id);
        // Set the session parameters
        self.set_bc_dests();

        let message =  self.data_manager.initialize_data(peer_id).unwrap_or_else(panic!("failed to initialize"));

        Ok(self.generate_relay_message(&message));
    }

    fn handle_error_response(&mut self, err_msg: &str) -> Result<ClientMessage, &'static str>{
        match  err_msg{
            resp if resp == String::from(NOT_YOUR_TURN) => {
                println!("not my turn");
                // wait
                let wait_time = time::Duration::from_millis(self.timeout as u64);
                thread::sleep(wait_time);
                println!("sending again");
                let msg = self.last_message.clone().unwrap();
                //TODO handle None
                return Ok(msg)
            },
            _ => {return Err(err_msg)}
        }
    }

    fn handle_server_response(&mut self, msg: &ServerMessage) -> Result<ClientMessage, &'static str>{
        let server_response = msg.response.unwrap();
        match server_response
            {
                ServerResponse::Register(peer_id) => {
                    let client_message = self.handle_register_response(peer_id);
                    match client_message{
                        Ok(_msg) => return Ok(_msg),
                        Err(e) => return Ok(ClientMessage::new()),
                    }
                },
                ServerResponse::ErrorResponse(err_msg) => {
                    println!("got error response");
                    let err_msg_slice: &str = &err_msg[..];
                    let msg = self.handle_error_response(err_msg_slice);
                    match msg {
                        Ok(_msg) => return Ok(_msg),
                        Err(e) => return Ok(ClientMessage::new()),
                    }
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


}


#[derive(Debug)]
pub enum ServerMessageType { // TODO this is somewhat duplicate
Response,
    Abort,
    RelayMessage,
    Undefined,
}

pub fn resolve_server_msg_type(msg: ServerMessage) -> ServerMessageType{
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



enum MessagePayloadType {
    /// Types of expected relay messages
    /// for step 0 we expect PUBLIC_KEY_MESSAGE
    /// for step 1 we expect COMMITMENT
    /// for step 2 we expect R_MESSAGE
    /// for step 3 we expect SIGNATURE

    PUBLIC_KEY(String), //  Serialized key
    COMMITMENT(String), //  Commitment
    R_MESSAGE(String),  //  (R,blind) of the peer
    SIGNATURE(String),  //  S_j
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


    let mut session = ProtocolSession::new(PROTOCOL_IDENTIFIER_ARG, PROTOCOL_CAPACITY_ARG, &message);
    let client = _tcp.and_then(|stream| {
        println!("sending register message");
        let framed_stream = stream.framed(ClientToServerCodec::new());

        // prepare register message -- TODO move this to session
        let mut msg = session.generate_register_message();


        // send register message to server
        let send_ = framed_stream.send(msg);
        send_.and_then(|stream| {
            let (tx, rx) = stream.split();
            let client = rx.and_then(|msg| {
                println!("Got message from server: {:?}", msg);
                let result = session.generate_client_answer(msg);
                match result {
                    Some(msg) => return Ok(msg),
                    None => return Ok(ClientMessage::new()),
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

