/// Implementation of a client that communicates with the relay server
/// this implememnataion is simplistic and used for POC and development and debugging of the
/// server
use std::env;
use std::net::SocketAddr;

use std::sync::{Arc, Mutex};
use tokio::codec::Framed;
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

#[derive(Default, Debug, Clone)]
struct Client {
    pub session: ProtocolSession,
}

impl Client {
    pub fn new() -> Client {
        Client {
            session: ProtocolSession::new(),
        }
    }
}

pub enum MessageProcessResult {
    Message,
    NoMessage,
    Abort,
}

impl Client {
    pub fn respond_to_server<E: 'static>(
        &mut self,
        msg: ServerMessage,
        // A sender to pass messages to be written back to the server
        tx: mpsc::Sender<ClientMessage>,
    ) -> Box<dyn Future<Item = (), Error = E>> {
        let response = self.handle_server_response(&msg).unwrap();
        println!("Returning {:?}", response);
        if response.is_empty() {
            Box::new(futures::future::ok(()))
        } else {
            Box::new(tx.clone().send(response.clone()).then(|_| Ok(())))
        }
    }

    pub fn handle_server_response(
        &mut self,
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
                        return Ok(client_message);
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

    pub fn generate_register_message(&mut self) -> ClientMessage {
        let mut msg = ClientMessage::new();
        msg.register(self.session.protocol_id.clone(), 2);
        msg
    }
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    // Parse what address we're going to connect to
    let addr = args
        .first()
        .unwrap_or_else(|| panic!("This program requires at least one argument"));

    let addr = addr.parse::<SocketAddr>().unwrap();

    // Create the event loop and initiate the connection to the remote server
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let tcp = TcpStream::connect(&addr, &handle);

    let session: std::sync::Arc<std::sync::Mutex<Client>> = Arc::new(Mutex::new(Client::new()));

    let handshake = tcp.and_then(|stream| {
        let handshake_io = Framed::new(stream, ClientToServerCodec::new(false));
        let mut client = session.lock().unwrap();
        let msg = client.generate_register_message();
        handshake_io
            .send(msg)
            .map(|handshake_io| handshake_io.into_inner())
            .map_err(|e| e.into())
    });

    let client = handshake.and_then(|socket| {
        let mut client = session.lock().unwrap();
        let _msg = client.generate_register_message();

        let (to_server, from_server) = Framed::new(socket, ClientToServerCodec::new(false)).split();
        let (tx, rx) = mpsc::channel(0);
        let reader = from_server.for_each(move |msg| {
            println!("Received {:?}", msg);
            client.respond_to_server(msg, tx.clone())
        });

        let writer = rx
            .map_err(|()| unreachable!("rx can't fail"))
            .fold(to_server, |to_server, msg| to_server.send(msg))
            .map(|_| ());

        reader
            .select(writer)
            .map(|_| println!("Closing connection"))
            .map_err(|(err, _)| err.into())
    });

    core.run(client).unwrap();
}
