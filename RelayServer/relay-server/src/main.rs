#![feature(refcell_replace_swap)]
///
/// Implementation of a server designed to work
/// as a relay between Peers communicating in a MPC protocol.
/// A protocol is represented by a unique identifier and a capacity
/// A client that wishes to communicate with othr peers via the server
/// must build its messages in the Codec supplied in relay_server_common lib
///
///
/// To run the server: run this main.rs file and in another terminal, run:
///     cargo +nightly run --example connect 127.0.0.1:8080
/// this will run a client that utilizes the server in some way
/// (for a run demo - run 2 clients)
///
extern crate futures;
extern crate tokio_core;
extern crate relay_server_common;


use std::env;

use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use std::net::SocketAddr;

use tokio_core::io::Io;
use tokio_core::reactor::Core;
use tokio_core::net::TcpListener;

use futures::{Stream, Sink, Future};
use futures::stream;
use futures::sync::mpsc;

use relay_server_common::{
    ServerToClientCodec, 
    ClientMessage,
    ServerMessage,
    PeerIdentifier, 
    ProtocolIdentifier,
    ServerResponse,
    RelayMessage,
    RegisterMessage,
    AbortMessage,
};

use relay_server_common::protocol::{
    ProtocolDescriptor
};

use relay_server_common::common::{
    RELAY_ERROR_RESPONSE,
    CANT_REGISTER_RESPONSE,
};


// Debug helper function to show type of a future
fn _debugf<F: Future<Item = (), Error = ()>>(_: F) {}

#[derive(Debug)]
pub enum ClientMessageType {
    Register,
    Abort,
    RelayMessage,
    Undefined,
}

// Represents the communication channel to remote client
#[derive(Clone, Debug)]
pub struct Client {
    tx:  mpsc::Sender<ServerMessage>,
}

impl Client {
    pub fn new(tx: mpsc::Sender<ServerMessage>) -> Client{
        Client {tx}
    }
}

#[derive(Clone, Debug)]
pub struct Peer {
    pub peer_id: PeerIdentifier,
    client: Client,
    pub registered: bool,
}


impl Peer {
    pub fn new(client: Client) -> Peer {
        Peer{
            peer_id: 0,
            client,
            registered: false
        }
    }

}

#[derive(Debug, Clone)]
pub enum RelaySessionState {
    Empty,

    Uninitialized,

    Initialized,

    Aborted,

}

#[derive(Debug, Clone)]
struct RelaySession {
    peers: Rc<RefCell<HashMap<SocketAddr, Peer>>>,

    active_peers: RefCell<u32>,

    protocol: RefCell<ProtocolDescriptor>,

    state: RefCell<RelaySessionState>,

}

impl RelaySession{
    /*
        RelaySession Private functions
    */

    /// Register a new peer to this relay session
    /// after adding this address as a peer,
    /// the state might change to either Uninitialized (if this is the first peer registering)
    /// or Initialized (meaning session has reached the required # of participants)
    fn register_new_peer(&self, addr: SocketAddr, protocol_id: ProtocolIdentifier, capacity: u32) -> Option<u32> {
        let _addr = &addr;
        let number_of_active_peers = self.peers.borrow().values().filter(|p| p.registered).fold(0, |acc, _| acc + 1);
        let protocol_descriptor = ProtocolDescriptor::new(protocol_id, capacity);
        match self.can_register(_addr, protocol_descriptor){
            true => {

                let mut peers = self.peers.borrow_mut();
                let peer = peers.get_mut(_addr).unwrap_or_else(|| panic!("No conection"));

                // activate this connection as a peer
                peer.registered = true;
                peer.peer_id = number_of_active_peers + 1;

                // if needed, set the ProtocolDescriptor for this sessuib
                // and change the state
                match self.state.clone().into_inner() {
                    RelaySessionState::Empty => {
                        self.protocol.replace(ProtocolDescriptor::new(protocol_id, capacity));
                        self.state.replace(RelaySessionState::Uninitialized);
                    },
                    _ => {}
                }
                if self.protocol.clone().into_inner().capacity == number_of_active_peers + 1 {
                    self.state.replace(RelaySessionState::Initialized);
                }
                return Some(number_of_active_peers + 1); //peer_id
            },
            false => {
                println!("unable to register {:}", addr); // error
                None
            }
        }
    }

    /// Checks if it is possible for this address
    /// to register as a peer in this session
    fn can_register(&self, addr: &SocketAddr, protocol: ProtocolDescriptor) -> bool{
        match self.state.clone().into_inner() {
            // if this is the first peer to register
            // check that the protocol is valid
            RelaySessionState::Empty => {
                println!("checking if protocol description is valid");
                if !relay_server_common::protocol::is_valid_protocol(&protocol){
                    return false;
                }
            },
            // if there is already a set protocol,
            // check that the peer wants to register to the set protocol
            RelaySessionState::Uninitialized => {
                println!("checking if protocol description is same as aet protocol description");
                let prot = self.protocol.clone().into_inner();
                if !(prot.id == protocol.id && prot.capacity == protocol.capacity){
                    return false;
                }
            },
            _ => {
                println!("Relay session state is neither empty nor uninitialized ");
                return false
            },
        }
        // register the peer iff it has an active connection and did not register yet
        if let Some(peer) = self.peers.borrow().get(addr) {
            return !peer.registered;
        }
        false
    }


    /// check if this relay message sent from the give SocketAddr
    /// is valid to send to rest of the peers
    fn can_relay(&self, from: &SocketAddr, msg: &RelayMessage) -> bool{

        println!("Checking if {:} can relay", msg.peer_number);
        println!("Server state: {:?}", self.state.clone().into_inner());
        println!("Turn of peer #: {:}", self.protocol.clone().into_inner().turn.clone().into_inner());

        match self.state.clone().into_inner() {
            RelaySessionState::Initialized => {
                println!("Relay sessions state is initialized");
            },
            _ => {
                println!("Relay sessions state is not initialized");
                return false
            },
        }
        // validate the sender in the message (peer_number field) is the peer associated with this address
        let sender = msg.peer_number;
        let mut peers = self.peers.borrow_mut();
        let peer = peers.get_mut(from);

        // if peer is present and registered
        if let Some(p) = peer {
            if p.registered && p.peer_id == sender {
                // check if it is this peers turn
                return self.protocol.clone().into_inner().next() == p.peer_id
            }
        }
        false
    }


}

impl RelaySession {
    /*
        RelaySession Public functions
    */

    /// Creates a new Relay Session with default (empty) fields
    /// and an Empty state
    pub fn new() -> RelaySession {
        RelaySession {

            peers: /*Arc::new(Mutex::new(HashMap::new())),// */(Rc::new(RefCell::new(HashMap::new()))),

            active_peers: RefCell::new(0),

            protocol: RefCell::new(relay_server_common::protocol::ProtocolDescriptor::new(0,2)),

            state: RefCell::new(RelaySessionState::Empty),

        }
    }

    /// Inserts a new connection to the session.
    /// the connection is NOT an active peer until it is registered to the session
    /// by sending a register message
    pub fn insert_new_connection(&self, addr: SocketAddr, client: Client) {
        self.peers.borrow_mut().insert(addr, Peer::new(client));
    }


    /// Removes a connection from the peers collection
    pub fn remove(&self, addr: &SocketAddr) -> Option<Peer> {
        self.peers.borrow_mut().remove(addr)
    }

    /// Send a message from the server to a group of peers
    /// takes each peers 'tx' part of the mpsc channel, and uses it to send the message to the client
    /// this peer represents
    pub fn multiple_send<E: 'static>(&self, message: ServerMessage, to: &Vec<PeerIdentifier>) -> Box<Future<Item = (), Error = E>> {
        let peers = self.peers.borrow();
        // For each client, clone its `mpsc::Sender` (because sending consumes the sender) and
        // start sending a clone of `message`. This produces an iterator of Futures.
        //let all_sends = client_map.values().map(|client| client.tx.clone().send(message.clone()));
        let sends = peers.values().filter(|peer|{
            let id = &(peer.peer_id as PeerIdentifier);
            to.contains(id ) && peer.registered
        }).map(|peer|{
            peer.client.tx.clone().send(message.clone())
        });

        let send_stream = stream::futures_unordered(sends).then(|_| Ok(()));

        // Convert the stream to a future that runs all the sends and box it up.
        Box::new(send_stream.for_each(|()| Ok(())))
    }

    /// try to send a server response message
    pub fn send_response<E: 'static>(&self, addr: SocketAddr,response: ServerMessage) -> Box<Future<Item = (), Error = E>> /*-> Result<(),()>*/ {
        let peers = self.peers.borrow();
        // For each client, clone its `mpsc::Sender` (b ecause sending consumes the sender) and
        // start sending a clone of `message`. This produces an iterator of Futures.
        //let all_sends = client_map.values().map(|client| client.tx.clone().send(message.clone()));
        if let Some(_peer)= peers.get(&addr){
            let mut to = Vec::new();
            if let Some(peer) = peers.get(&addr) {
                to.push(peer.peer_id as PeerIdentifier);
            }
            self.multiple_send(response, &to)
        }else {
            panic!("err")
        }
    }

    /// try to send a relay message to the desired peers
    pub fn relay_message<E: 'static>(&self, from: &SocketAddr, msg: RelayMessage) -> Box<Future<Item = (), Error = E>>{
        let mut server_msg = ServerMessage::new();
        let mut _to = vec![];
        let peer_id = self.peers.borrow_mut().get_mut(from).unwrap().peer_id;
        if self.can_relay(from, &msg) {
            server_msg.relay_message = Some(msg.clone());
            _to = msg.to;
            let sender_index = _to.iter().position(|x| *x == peer_id);
            if sender_index.is_some(){
                _to.remove(sender_index.unwrap());
            }
            self.protocol.clone().into_inner().advance_turn();

            println!("sending relay message: {:?}", server_msg);
            println!("sending to: {:?}", _to);
        }
        else {
            // send an error response to sender
            server_msg.response = Some(ServerResponse::ErrorResponse(String::from(RELAY_ERROR_RESPONSE)));
            _to = vec![peer_id];
        }
        self.multiple_send(server_msg, &_to)
    }
    /// try to register a new peer
    pub fn register<E: 'static>(&self, addr: SocketAddr, protocol_id: ProtocolIdentifier, capacity: u32)  -> Box<Future<Item = (), Error = E>> {
        let mut server_msg = ServerMessage::new();
        let peer_id = self.register_new_peer(addr, protocol_id, capacity);
        if peer_id.is_some() {
            server_msg.response = Some(ServerResponse::Register(peer_id.unwrap()));
        } else {
            server_msg.response = Some(ServerResponse::ErrorResponse(String::from(CANT_REGISTER_RESPONSE)));
        }
        self.send_response(addr, server_msg)
    }

    /// abort this relay session and send abort message to all peers
    pub fn abort<E: 'static>(&self, addr:SocketAddr) -> Box<Future<Item = (), Error = E>> {
        let mut server_msg = ServerMessage::new();
        match self.state.clone().into_inner(){
            RelaySessionState::Initialized => {},
            _ => {return Box::new(futures::future::ok(()))},
        }
        let peer= self.get_peer(&addr);
        match peer {
            Some(p) => {
                server_msg.abort = Some(AbortMessage::new(p.peer_id, self.protocol.clone().into_inner().id));
                self.state.replace(RelaySessionState::Aborted);
                let mut to = Vec::new();
                let peers = self.peers.borrow();
                peers.values().filter(|p| p.registered).for_each(|p| {
                    to.push(p.peer_id);
                });
                self.multiple_send(server_msg, &to)
            },
            None => {
               Box::new(futures::future::ok(()))
            }
        }
    }

    /// handle a closed connection
    /// if it an active peer disconnected - abort the session
    /// otherwise, simply remove the connection of this address from the peers collection
    pub fn connection_closed<E: 'static>(&mut self, addr:SocketAddr) -> Box<Future<Item = (), Error = E>>{
        let mut to= Vec::new();
        self.peers.borrow().values().filter(|p| p.registered).for_each(|p| {
            to.push(p.peer_id);
        });
        let peers = self.peers.borrow();
        // check if the address was a peer
        let peer= peers.get(&addr);
        let mut peer_disconnected = false;
        let mut peer_id = 0;
        if peer.is_some(){
            let p = peer.unwrap();
            if !p.registered {
                self.remove(&addr);
                return Box::new(futures::future::ok(()));
            }
            else{
               peer_id = p.peer_id;
               peer_disconnected = true;
            }
        }
        if peer_disconnected {
            let mut server_msg= ServerMessage::new();
            server_msg.abort = Some(AbortMessage::new(peer_id, self.protocol.clone().into_inner().id));
            self.state.replace(RelaySessionState::Aborted);
            return self.multiple_send(server_msg, &to);
        }
        Box::new(futures::future::ok(()))
    }


    /// get a copy of Peer that addr represents
    pub fn get_peer(&self, addr: &SocketAddr) -> Option<Peer> {
        match self.peers.borrow().get(addr) {
            Some(p) => {
                match p.registered {
                    true => Some(p.clone()),
                    false => None
                }
            },
            None => None
        }
    }

}

/// get the message type of a given client message
pub fn resolve_msg_type(msg: ClientMessage) -> ClientMessageType {

    if msg.register.is_some(){
        return ClientMessageType::Register;
    }
    if msg.relay_message.is_some(){
        return ClientMessageType::RelayMessage;
    }
    if msg.abort.is_some(){
        return ClientMessageType::Abort;
    }
    return ClientMessageType::Undefined;
}

fn main() {
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());
    let addr = addr.parse().unwrap();

    // Create the event loop and TCP listener we'll accept connections on.
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let listener= TcpListener::bind(&addr, &handle).unwrap();
    println!("Listening on: {}", addr);

    // Create the session fot the relay server
    let relay_session = Arc::new(Mutex::new(RelaySession::new()));

    let srv = listener.incoming().for_each(move |(socket, addr)| {
        // Got a new connection
        println!("server got a new connection");

        // Frame the socket with JSON codec
        let framed_socket = socket.framed(ServerToClientCodec::new());

        // obtain a clone of the RelaySession
        let relay_session_inner = Arc::clone(&relay_session);//relay_session.clone();

        // create a channel of communication with the (potential) peer
        let (tx, rx) = mpsc::channel(0);//(8);

        // insert this client to the servers active_connections
        relay_session_inner.lock().unwrap().insert_new_connection(addr.clone(),Client::new(tx));

        // split the socket to reading part (stream) and writing part (sink)
        let (to_client, from_client) = framed_socket.split();

        // define future for receiving half
        let relay_session_inner = Arc::clone(&relay_session);
        let reader = from_client.for_each(move |msg| {
            let relay_session_i= relay_session_inner.lock().unwrap();
            let relay_session_inner = &*relay_session_i;

            println!("got message: {:?}", msg);
            let msg_type = resolve_msg_type(msg.clone());
            println!("message type is {:?}", msg_type);

            // this is our main logic for receiving messages from peer
            match msg_type {
                ClientMessageType::Register => {
                    let register = msg.register.unwrap();
                    println!("got register message. protocol id requested: {}", register.protocol_id);
                    relay_session_inner.register(addr, register.protocol_id, register.capacity)
                },
                ClientMessageType::RelayMessage => {
                    let peer = relay_session_inner.get_peer(&addr).unwrap_or_else(||
                        panic!("not a peer"));
                    println!("got relay message from {}", peer.peer_id);
                    let relay_msg = msg.relay_message.unwrap().clone();
                    relay_session_inner.relay_message(&addr, relay_msg)
                },
                ClientMessageType::Abort => {
                    let peer = relay_session_inner.get_peer(&addr).unwrap_or_else(|| panic!("not a peer"));
                    println!("got abort message from {}", peer.peer_id);
                    relay_session_inner.abort(addr)
                },
                ClientMessageType::Undefined => {
                    //panic!("Got unknown or empty message");
                    Box::new(futures::future::ok(())) // TODO this disconnects?
                }
            }
        });

        // define future for sending half
        let writer = rx
            .map_err(|()| unreachable!("rx can't fail"))
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
        let connection = reader.select(writer);

        // map & map_err here are used for the case reading half or writing half is dropped
        // in which case we will be dropping the other half as well
        let relay_session_inner = Arc::clone(&relay_session);
        handle.spawn(connection.map(|_| ()).map_err(|(err, _)| err)
            .then(move |_| {
                // connection is closed
                println!("Disconnected");

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


