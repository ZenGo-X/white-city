use serde_json::{Error, Map, Value};
use std::net::SocketAddr;
use subtle_encoding::base64;
use tendermint::rpc::Client;

use relay_server_common::{
    ClientMessage, ClientToServerCodec, PeerIdentifier, ProtocolIdentifier, RelayMessage,
    ServerMessage, ServerMessageType, ServerResponse,
};

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

    let client = Client::new(&"tcp://127.0.0.1:26657".parse().unwrap()).unwrap();

    let mut msg = ClientMessage::new();
    let client_addr: SocketAddr = format!("127.0.0.1:808{}", 0).parse().unwrap();
    msg.register(client_addr, 0, 2);

    println!("Regsiter message {:?}", msg);
    let tx = tendermint::abci::transaction::Transaction::new(serde_json::to_string(&msg).unwrap());
    let response = client.broadcast_tx_commit(tx).unwrap();

    println!("{:?}", response);
}
