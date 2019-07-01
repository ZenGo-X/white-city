use futures::stream;
use futures::sync::mpsc;
use futures::{Future, Sink, Stream};
use log::{debug, error, info, warn};
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::codec::Framed;
use tokio_core::net::TcpListener;
use tokio_core::reactor::Core;

use crate::relay_session::{Client, RelaySession};
use relay_server_common::{ClientMessageType, ServerMessage, ServerToClientCodec};

pub struct RelayServer {
    pub rs: Option<RelaySession>,
    addr: std::net::SocketAddr,
}

impl RelayServer {
    pub fn new(addr: SocketAddr) -> RelayServer {
        RelayServer {
            rs: None,
            addr: addr,
        }
    }

    /// Starts the relay server
    pub fn start_server(&self, capacity: u32) {
        // Create the event loop and TCP listener we'll accept connections on.
        let mut core = Core::new().unwrap();
        let handle = core.handle();

        let listener = TcpListener::bind(&self.addr, &handle).unwrap();
        info!("Listening on: {}", &self.addr);

        // Create the session fot the relay server
        // TODO: Relay sessions should start when a new client connects
        let relay_session = Arc::new(RelaySession::new(capacity));

        let srv = listener.incoming().for_each(move |(socket, addr)| {
            // Got a new connection
            info!("Server got a new connection");

            // Frame the socket with JSON codec
            //let framed_socket = ServerToClientCodec::new(false).framed(socket);
            let framed_socket = Framed::new(socket, ServerToClientCodec::new(false));

            // obtain a clone of the RelaySession
            let relay_session_inner = Arc::clone(&relay_session); //relay_session.clone();

            // create a channel of communication with the (potential) peer
            let (tx, rx) = mpsc::channel(0);

            // insert this client to the servers active_connections
            relay_session_inner.insert_new_connection(addr.clone(), Client::new(tx));

            // split the socket to reading part (stream) and writing part (sink)
            let (to_client, from_client) = framed_socket.split();

            // define future for receiving half
            let relay_session_inner = Arc::clone(&relay_session);
            let reader = from_client.for_each(move |msg| {
                let msg_type = msg.msg_type();

                // this is our main logic for receiving messages from peer
                match msg_type {
                    ClientMessageType::Register => {
                        let register = msg.register.unwrap();
                        info!(
                            "Got register message. protocol id requested: {}",
                            register.protocol_id
                        );
                        let messages_to_send = relay_session_inner.register(
                            addr,
                            register.protocol_id,
                            register.capacity,
                        );
                        RelayServer::send_messages(&messages_to_send)
                    }
                    ClientMessageType::RelayMessage => {
                        let peer = relay_session_inner
                            .get_peer_by_address(&addr)
                            .unwrap_or_else(|| panic!("not a peer"));
                        info!("Got relay message from {}", peer.peer_id);
                        let relay_msg = msg.relay_message.unwrap().clone();
                        relay_session_inner.relay_message(&addr, relay_msg)
                    }
                    ClientMessageType::Abort => {
                        let peer = relay_session_inner
                            .get_peer_by_address(&addr)
                            .unwrap_or_else(|| panic!("not a peer"));
                        debug!("Got abort message from {}", peer.peer_id);
                        let messages_to_send = relay_session_inner.abort(addr);
                        RelayServer::send_messages(&messages_to_send)
                    }
                    ClientMessageType::Test => {
                        let sender = relay_session_inner
                            .get_sender_by_address(&addr)
                            .unwrap_or_else(|| panic!("not a peer"));
                        let msg = ServerMessage::new();
                        RelayServer::send_response(sender, msg)
                    }
                    ClientMessageType::Undefined => {
                        warn!("Got unknown or empty message");
                        let messages_to_send = relay_session_inner.abort(addr);
                        RelayServer::send_messages(&messages_to_send)
                    }
                }
            });

            // define future for sending half
            let writer = rx
                .map_err(|()| unreachable!("rx can't fail"))
                // fold on a stream (rx) takes an initial value (to_client, a Sink)
                // and run the given closure, for each value passed from the stream (message to send to
                // the client)
                .fold(to_client, |to_client, msg| to_client.send(msg))
                // this map will cleanly drop the writing half of the socket when done with all processing
                .map(|_| ());

            // if any of the reading/writing half is done - the whole connection is finished
            // this makes select a sensible combinator
            let connection = reader.select(writer);

            // map & map_err here are used for the case reading half or writing half is dropped
            // in which case we will be dropping the other half as well
            let relay_session_inner = Arc::clone(&relay_session);
            handle.spawn(
                connection
                    .map(|_| ())
                    .map_err(|(err, _)| {
                        error!("\nERROR OCCURED: {:?}", err);
                        err
                    })
                    .then(move |_| {
                        // connection is closed
                        warn!("Disconnected");

                        // this means either a peer disconnected - same as abort,
                        // or an active connection closed - which is allowed
                        relay_session_inner.connection_closed(addr)
                    }),
            );

            Ok(())
        });

        // execute server
        core.run(srv).unwrap();
    }

    pub fn send_messages<E: 'static>(
        messages_to_send: &Vec<(ServerMessage, mpsc::Sender<ServerMessage>)>,
    ) -> Box<dyn Future<Item = (), Error = E>> {
        let sends = messages_to_send
            .iter()
            .map(|(msg, sink)| sink.clone().send(msg.clone()));
        let send_stream = stream::futures_unordered(sends).then(|_| Ok(()));
        Box::new(send_stream.for_each(|()| Ok(())))
    }

    pub fn send_response<E: 'static>(
        tx: mpsc::Sender<ServerMessage>,
        response: ServerMessage,
    ) -> Box<dyn Future<Item = (), Error = E>> {
        let sends = vec![tx.clone().send(response.clone())];
        let send_stream = stream::futures_unordered(sends).then(|_| Ok(()));
        Box::new(send_stream.for_each(|()| Ok(())))
    }
}
