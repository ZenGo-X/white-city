use serde_json::{Error, Map, Value};
use std::net::SocketAddr;
use subtle_encoding::base64;
use tendermint::rpc::Client;

use relay_server_common::{
    ClientMessage, PeerIdentifier, ProtocolIdentifier, RelayMessage, ServerMessage,
    ServerMessageType, ServerResponse,
};

use clap::{App, Arg, ArgMatches};

fn arg_matches<'a>() -> ArgMatches<'a> {
    App::new("relay-server")
        .arg(
            Arg::with_name("index")
                .short("I")
                .long("index")
                .default_value("1"),
        )
        .arg(
            Arg::with_name("capacity")
                .default_value("2")
                .short("P")
                .long("capacity"),
        )
        .get_matches()
}

// ClientSession holds session data
#[derive(Default, Debug, Clone)]
struct ProtocolSession {
    pub registered: bool,
    pub peer_id: PeerIdentifier,
    pub protocol_id: ProtocolIdentifier,
    pub next_message: Option<ClientMessage>,
}

impl ProtocolSession {
    pub fn new() -> ProtocolSession {
        ProtocolSession {
            registered: false,
            peer_id: 0,
            protocol_id: 0,
            next_message: None,
        }
    }
}

struct SessionClient {
    pub session: ProtocolSession,
}

impl SessionClient {
    pub fn new() -> SessionClient {
        SessionClient {
            session: ProtocolSession::new(),
        }
    }
}

pub enum MessageProcessResult {
    Message,
    NoMessage,
    Abort,
}

impl SessionClient {
    pub fn handle_server_response(
        &self,
        msg: &ServerMessage,
    ) -> Result<ClientMessage, &'static str> {
        println!("Got message from server: {:?}", msg);
        let msg_type = msg.msg_type();
        match msg_type {
            ServerMessageType::Response => {
                // we expect to receive a register response here
                let server_response = msg.response.clone().unwrap();
                match server_response {
                    ServerResponse::Register(peer_id) => {
                        println!("Peer identifier: {}", peer_id);
                        // create a mock relay message
                        let mut client_message = ClientMessage::new();
                        let mut relay_message =
                            RelayMessage::new(peer_id, self.session.protocol_id);
                        let mut to: Vec<u32> = Vec::new();
                        if peer_id == 2 {
                            to.push(1);
                        } else {
                            to.push(2);
                        }

                        relay_message.set_message_params(to, format!("Hi from {}", peer_id));
                        client_message.relay_message = Some(relay_message.clone());
                        Ok(client_message)
                    }
                    _ => panic!("failed to register"),
                }
            }
            ServerMessageType::RelayMessage => {
                println!("Got new relay message");
                println!("{:?}", msg.relay_message.clone().unwrap());
                //Ok(MessageProcessResult::NoMessage)
                Ok(ClientMessage::new())
            }
            ServerMessageType::Abort => {
                println!("Got abort message");
                //Ok(MessageProcessResult::NoMessage)
                Ok(ClientMessage::new())
            }

            ServerMessageType::Undefined => Ok(ClientMessage::new()),
        }
    }

    pub fn generate_register_message(&self) -> ClientMessage {
        let mut msg = ClientMessage::new();
        let client_addr: SocketAddr = format!("127.0.0.1:808{}", 0).parse().unwrap();
        msg.register(client_addr, self.session.protocol_id, 2);
        msg
    }
}

fn main() {
    better_panic::Settings::debug()
        .most_recent_first(false)
        .lineno_suffix(true)
        .install();

    let matches = arg_matches();

    let index: u32 = matches
        .value_of("index")
        .unwrap()
        .parse()
        .expect("Unable to parse index");

    let capacity: u32 = matches
        .value_of("capacity")
        .unwrap()
        .parse()
        .expect("Invalid number of participants");

    let client = Client::new(&"tcp://127.0.0.1:26657".parse().unwrap()).unwrap();

    let mut msg = ClientMessage::new();
    let client_addr: SocketAddr = format!("127.0.0.1:808{}", index).parse().unwrap();
    msg.register(client_addr, 0, capacity);

    println!("Regsiter message {:?}", msg);
    let tx = tendermint::abci::transaction::Transaction::new(serde_json::to_string(&msg).unwrap());
    let response = client.broadcast_tx_commit(tx).unwrap();
    let client_index = response.clone().deliver_tx.log.unwrap();
    println!("Registered OK");
    println!("Index {:?}", client_index);
    let client_index: u32 = client_index.to_string().parse::<u32>().unwrap();
    println!("Index {:?}", client_index);

    let mut msg = ClientMessage::new();
    msg.register(client_addr, 0, capacity);
}
