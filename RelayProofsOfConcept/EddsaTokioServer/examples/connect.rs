///
/// Implementation of a client that communicates with the relay server
/// this implememnataion is simplistic and used for POC and development and debugging of the
/// server
///
///
extern crate futures;
extern crate relay_server_common;
extern crate tokio_core;

use std::env;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::{thread, time};

use tokio_core::io::Io;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Core;

use futures::sync::mpsc;
use futures::{Future, Sink, Stream};

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

pub enum MessageProcessResult {
    Message,
    NoMessage,
    Abort,
}

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    // Parse what address we're going to co nnect to
    let addr = args
        .first()
        .unwrap_or_else(|| panic!("this program requires at least one argument"));

    let addr = addr.parse::<SocketAddr>().unwrap();

    // Create the event loop and initiate the connection to the remote server
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let _tcp = TcpStream::connect(&addr, &handle);

    let mut session = ProtocolSession::new();

    let client = _tcp
        .and_then(|stream| {
            println!("sending register message");

            let framed_stream = stream.framed(ClientToServerCodec::new());

            // define which protocol id we would like to participate in
            let protocol_id: ProtocolIdentifier = 1;
            let capacity: u32 = 2;

            // prepare register message
            let mut msg = ClientMessage::new();

            let register_msg = msg.register(protocol_id, capacity);
            session.protocol_id = 1;

            // send register message to server
            framed_stream.send(msg).and_then(|stream| {
                let (tx, rx) = stream.split();
                let client = rx
                    .and_then(|msg| {
                        println!("Got message from server: {:?}", msg);
                        let msg_type = resolve_server_msg_type(&msg);
                        match msg_type {
                            ServerMessageType::Response => {
                                // we expect to receive a register response here
                                let server_response = msg.response.unwrap();
                                match server_response {
                                    ServerResponse::Register(peer_id) => {
                                        println!("Peer identifier: {}", peer_id);
                                        // create a mock relay message
                                        let mut client_message = ClientMessage::new();
                                        let mut relay_message = RelayMessage::new(peer_id, 1);
                                        let mut to: Vec<u32> = Vec::new();
                                        if peer_id == 2 {
                                            to.push(1);
                                        } else {
                                            to.push(2);
                                        }

                                        // wait a little so we can spawn the second client
                                        let wait_time = time::Duration::from_millis(5000);
                                        thread::sleep(wait_time);

                                        relay_message
                                            .set_message_params(to, format!("Hi from {}", peer_id));
                                        client_message.relay_message = Some(relay_message.clone());
                                        //session.next_message = Some(client_message);
                                        return Ok(client_message);
                                    }
                                    _ => panic!("failed to register"),
                                }
                            }
                            ServerMessageType::RelayMessage => {
                                println!("Got new relay message");
                                println!("{:?}", msg.relay_message.unwrap());
                                //Ok(MessageProcessResult::NoMessage)
                                Ok(ClientMessage::new())
                            }
                            ServerMessageType::Abort => {
                                println!("Got abort message");
                                //Ok(MessageProcessResult::NoMessage)
                                Ok(ClientMessage::new())
                            }
                            ServerMessageType::Undefined => {
                                Ok(ClientMessage::new())
                                //panic!("Got undefined message: {:?}",msg);
                            }
                        }
                    })
                    .forward(tx);
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

    core.run(client); //.unwrap();
}
