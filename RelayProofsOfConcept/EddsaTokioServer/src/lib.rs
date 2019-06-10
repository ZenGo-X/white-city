extern crate chrono;
extern crate futures;
extern crate relay_server_common;
extern crate structopt;
extern crate tokio;
extern crate tokio_core;

use futures::sync::mpsc;
use futures::{Future, Sink, Stream};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use tokio_core::io::Io;
use tokio_core::net::TcpListener;
use tokio_core::reactor::Core;

use relay_server_common::ServerToClientCodec;

mod relay_session;
pub use relay_session::{resolve_client_msg_type, Client, ClientMessageType, RelaySession};

/// Starts the relay server
pub fn start_server(addr: &SocketAddr, capacity: u32) {
    // Create the event loop and TCP listener we'll accept connections on.
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let listener = TcpListener::bind(&addr, &handle).unwrap();
    println!("\nListening on: {}", addr);

    // Create the session fot the relay server
    let relay_session = Arc::new(Mutex::new(relay_session::RelaySession::new(capacity)));

    let srv = listener.incoming().for_each(move |(socket, addr)| {
        // Got a new connection
        println!("\nserver got a new connection");

        // Frame the socket with JSON codec
        let framed_socket = socket.framed(ServerToClientCodec::new());

        // obtain a clone of the RelaySession
        let relay_session_inner = Arc::clone(&relay_session);//relay_session.clone();

        // create a channel of communication with the (potential) peer
        let (tx, rx) = mpsc::channel(0);//(8);

        // insert this client to the servers active_connections
        relay_session_inner.lock().unwrap().insert_new_connection(addr.clone(),relay_session::Client::new(tx));

        // split the socket to reading part (stream) and writing part (sink)
        let (to_client, from_client) = framed_socket.split();

        // define future for receiving half
        let relay_session_inner = Arc::clone(&relay_session);
        let reader = from_client.for_each(move |msg| {
            let relay_session_i= relay_session_inner.lock().unwrap();
            let relay_session_inner = &*relay_session_i;

            let msg_type = relay_session::resolve_client_msg_type(&msg);

            // this is our main logic for receiving messages from peer
            match msg_type {
                relay_session::ClientMessageType::Register => {
                    let register = msg.register.unwrap();
                    println!("\ngot register message. protocol id requested: {}", register.protocol_id);
                    relay_session_inner.register(addr, register.protocol_id, register.capacity)
                },
                relay_session::ClientMessageType::RelayMessage => {
                    let peer = relay_session_inner.get_peer(&addr).unwrap_or_else(||
                        panic!("not a peer"));
                    println!("\ngot relay message from {}", peer.peer_id);
                    let relay_msg = msg.relay_message.unwrap().clone();
                    relay_session_inner.relay_message(&addr, relay_msg)
                },
                relay_session::ClientMessageType::Abort => {
                    let peer = relay_session_inner.get_peer(&addr).unwrap_or_else(|| panic!("not a peer"));
                    println!("\ngot abort message from {}", peer.peer_id);
                    relay_session_inner.abort(addr)
                },
                relay_session::ClientMessageType::Undefined => {
                    println!("\nGot unknown or empty message");
                    relay_session_inner.abort(addr)//Box::new(futures::future::ok(())) // TODO this disconnects?
                }
            }
        });

        // define future for sending half
        let writer = rx
            .map_err(|()|unreachable!("rx can't fail"))
            // fold on a stream (rx) takes an initial value (to_client, a Sink)
            // and run the given closure, for each value passed from the stream (message to send to
            // the client)
            .fold(to_client, |to_client, msg| {
                to_client.send(msg)
            })
            // this map will cleanly drop the writing half of the socket when done with all processing
            .map(|_| ());


        // if any of the reading/writing half is done - the whole connection is finished
        // this makes select a sensible combinator
        let connection = reader./*map_err(|err|{println!("ERROR OCCURED IN READER:{:?}",err);err}).*/select(writer);

        // map & map_err here are used for the case reading half or writing half is dropped
        // in which case we will be dropping the other half as well
        let relay_session_inner = Arc::clone(&relay_session);
        handle.spawn(connection.map(|_| ()).map_err(|(err, _)| {println!("\nERROR OCCURED: {:?}",err);err})
            .then(move |_| {
                // connection is closed
                println!("\nDisconnected");

                // this means either a peer disconnected - same as abort,
                // or an active connection closed - which is allowed
                let mut relay_session_inner = relay_session_inner.lock().unwrap();
                relay_session_inner.connection_closed(addr)
            }));

        Ok(())
    });

    // execute server
    core.run(srv).unwrap();
}
