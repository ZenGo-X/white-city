//! Relay Server implementation
//! You can test this out by running:
//!     cargo run
//! And then in another window run:
//!
//!     cargo run --example connect 127.0.0.1:8080
//!

extern crate futures;
extern crate tokio_core;
extern crate tokio_io;

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::iter;
use std::env;
use std::io::{Error, ErrorKind, BufReader};

use futures::Future;
use futures::stream::{self, Stream};
use tokio_core::net::TcpListener;
use tokio_core::reactor::Core;
use tokio_io::io;
use tokio_io::AsyncRead;

struct Peer {
    tx: futures::sync::mpsc::UnboundedSender<String>,
    id: usize,
}

struct ProtocolSession {
    n: usize,
    connections: usize,
    activePeers: usize,
    round: usize,
}

impl ProtocolSession{
    pub fn new() -> ProtocolSession {
        ProtocolSession{
            n: 2,
            connections: 0,
            activePeers: 0,
            round: 0
        }
    }

    pub fn new_connection(&mut self) -> bool {
        if self.connections < self.n {
            self.connections += 1;
            return true;
        }
        false
    }

    pub fn increase_round(&mut self) -> bool {
        self.round = (self.round + 1) % self.n;
        true
    }

}
fn main() {
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());
    let addr = addr.parse().unwrap();

    // Create the event loop and TCP listener we'll accept connections on.
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let socket = TcpListener::bind(&addr, &handle).unwrap();
    println!("Listening on: {}", addr);

    // This is a single-threaded server, so we can just use Rc and RefCell to
    // store the map of all connections we know about.
    let connections = Rc::new(RefCell::new(HashMap::new()));
    let session = Rc::new(RefCell::new(ProtocolSession::new()));

    let srv = socket.incoming().for_each(move |(stream, addr)| {
        println!("New Connection: {}", addr);
        session.borrow_mut().new_connection();
        let session_inner = session.clone();
        let (reader, writer) = stream.split();

        // Create a channel for our stream, which other sockets will use to
        // send us messages. Then register our address with the stream to send
        // data to us.
        let (tx, rx) = futures::sync::mpsc::unbounded();

        let peer_number= connections.borrow_mut().keys().len() + 1;
        connections.borrow_mut().insert(addr, Peer {tx, id:peer_number});
        println!("peer number: {}", peer_number);

        // Define here what we do for the actual I/O. That is, read a bunch of
        // lines from the socket and dispatch them while we also write any lines
        // from other sockets.
        let connections_inner = connections.clone();
        let reader = BufReader::new(reader);

        // Model the read portion of this socket by mapping an infinite
        // iterator to each line off the socket. This "loop" is then
        // terminated with an error once we hit EOF on the socket.
        let iter = stream::iter_ok::<_, Error>(iter::repeat(()));
        let socket_reader = iter.fold(reader, move |reader, _| {
            // Read a line off the socket, failing if we're at EOF
            let line = io::read_until(reader, b'\n', Vec::new());
            let line = line.and_then(|(reader, vec)| {
                if vec.len() == 0 {
                    Err(Error::new(ErrorKind::BrokenPipe, "broken pipe"))
                } else {
                    Ok((reader, vec))
                }
            });

            // Convert the bytes we read into a string, and then send that
            // string to all other connected clients.
            let line = line.map(|(reader, vec)| {
                (reader, String::from_utf8(vec))
            });
            let connections = connections_inner.clone();
            let session = session_inner.clone();
            line.map(move |(reader, message)| {
                println!("{}: {:?}", addr, message);


                let mut conns = connections.borrow_mut();

                let mut protocol_session = session.borrow_mut();
                if let Ok(msg) = message {
                    // Check that it is the senders turn

                    let id = conns.get_mut(&addr).unwrap().id;

                    if id == protocol_session.round + 1 {
                        protocol_session.increase_round();
                    }
                    else {
                        println!("this is not this senders turn!");
                        return reader;
                    }

                    // For each open connection except the sender, send the
                    // string via the channel.
                    let iter = conns.iter_mut()
                        .filter(|&(&k, _)| k != addr)
                        .map(|(_, v)| v);
                    for peer in iter {
                        println!("sending to peer {}", peer.id);

                        peer.tx.unbounded_send(format!("{}: {}", addr, msg)).unwrap();
                    }
                } else {
                    let peer = conns.get_mut(&addr).unwrap();
                    peer.tx.unbounded_send("You didn't send valid UTF-8.".to_string()).unwrap();
                }
                reader
            })
        });

        // Whenever we receive a string on the Receiver, we write it to
        // `WriteHalf<TcpStream>`.
        let socket_writer = rx.fold(writer, |writer, msg| {
            println!("{:?}",writer);
            let amt = io::write_all(writer, msg.into_bytes());
            let amt = amt.map(|(writer, _)| writer);
            amt.map_err(|_| ())
        });

        // Now that we've got futures representing each half of the socket, we
        // use the `select` combinator to wait for either half to be done to
        // tear down the other. Then we spawn off the result.
        let connections = connections.clone();
        let socket_reader = socket_reader.map_err(|_| ());
        let connection = socket_reader.map(|_| ()).select(socket_writer.map(|_| ()));
        handle.spawn(connection.then(move |_| {
            connections.borrow_mut().remove(&addr);
            println!("Connection {} closed.", addr);
            Ok(())
        }));

        Ok(())
    });

    // execute server
    core.run(srv).unwrap();
}
