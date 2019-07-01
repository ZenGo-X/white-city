use futures::stream;
use futures::sync::mpsc;
use futures::{Future, Sink, Stream};
use log::{debug, info, warn};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use relay_server_common::{
    AbortMessage, PeerIdentifier, ProtocolIdentifier, RelayMessage, ServerMessage, ServerResponse,
};

use relay_server_common::common::{NOT_YOUR_TURN, STATE_NOT_INITIALIZED};

use relay_server_common::protocol::ProtocolDescriptor;

// Represents the communication channel to remote client
#[derive(Clone, Debug)]
pub struct Client {
    tx: mpsc::Sender<ServerMessage>,
}

impl Client {
    pub fn new(tx: mpsc::Sender<ServerMessage>) -> Client {
        Client { tx }
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
        Peer {
            peer_id: 0,
            client,
            registered: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelaySessionState {
    Empty,

    Uninitialized,

    Initialized,

    Aborted,
}

#[derive(Debug, Clone)]
pub struct RelaySession {
    peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>,

    active_peers: Arc<RwLock<u32>>,

    protocol: Arc<RwLock<ProtocolDescriptor>>,

    state: Arc<RwLock<RelaySessionState>>,
}

impl RelaySession {
    /// Returns the current number of active peers.
    /// If a peer disconnects, it should be removed from the active peers
    fn get_number_of_active_peers(&self) -> u32 {
        self.peers
            .read()
            .unwrap()
            .iter()
            .filter(|(_, p)| p.registered)
            .fold(0, |acc, _| acc + 1)
    }

    /// Register a new peer to this relay session
    /// after adding this address as a peer,
    /// the state might change to either Uninitialized (if this is the first peer registering)
    /// or Initialized (meaning session has reached the required # of participants)
    pub fn register_new_peer(
        &self,
        addr: SocketAddr,
        protocol_id: ProtocolIdentifier,
        capacity: u32,
    ) -> Option<u32> {
        let _addr = &addr;
        let number_of_active_peers = self.get_number_of_active_peers();

        let protocol_descriptor = ProtocolDescriptor::new(protocol_id, capacity);
        info!("-----------------PEERS: {:?}---------------", self.peers);
        match self.can_register(_addr, protocol_descriptor) {
            true => {
                let mut peers = self.peers.write().unwrap();
                let peer = peers
                    .get_mut(_addr)
                    .unwrap_or_else(|| panic!("No conection"));

                // activate this connection as a peer
                peer.registered = true;
                peer.peer_id = number_of_active_peers + 1;
                // if needed, set the ProtocolDescriptor for this sessuib
                // and change the state
                let state = self.state();
                match state {
                    RelaySessionState::Empty => {
                        self.set_protocol(ProtocolDescriptor::new(protocol_id, capacity));
                        self.set_state(RelaySessionState::Uninitialized);
                    }
                    _ => {}
                }
                //if self.protocol.clone().into_inner().capacity == number_of_active_peers + 1 {
                if self.protocol().capacity == number_of_active_peers + 1 {
                    self.set_state(RelaySessionState::Initialized);
                }
                return Some(number_of_active_peers + 1); //peer_id
            }
            false => {
                warn!("Unable to register {:}", addr); // error
                None
            }
        }
    }

    /// Checks if it is possible for this address
    /// to register as a peer in this session
    fn can_register(&self, addr: &SocketAddr, protocol: ProtocolDescriptor) -> bool {
        match self.state() {
            // if this is the first peer to register
            // check that the protocol is valid
            RelaySessionState::Empty => {
                debug!("Checking if protocol description is valid");
                if !relay_server_common::protocol::is_valid_protocol(&protocol) {
                    warn!("Protocol is invalid");

                    return false;
                }
            }
            // if there is already a set protocol,
            // check that the peer wants to register to the set protocol
            RelaySessionState::Uninitialized => {
                debug!("Checking if protocol description is same as at protocol description");
                let prot = self.protocol();
                if !(prot.id == protocol.id && prot.capacity == protocol.capacity) {
                    warn!("Protocol description does not fit current configuration");
                    return false;
                }
            }
            _ => {
                debug!("Relay session state is neither empty nor uninitialized ");
                return false;
            }
        }
        // register the peer iff it has an active connection and did not register yet
        if let Some(peer) = self.peers.read().unwrap().get(addr) {
            return !peer.registered;
        }
        false
    }

    /// check if this relay message sent from the given SocketAddr
    /// is valid to send to rest of the peers
    fn can_relay(&self, from: &SocketAddr, msg: &RelayMessage) -> Result<(), &'static str> {
        debug!("Checking if {:} can relay", msg.peer_number);
        debug!("Server state: {:?}", self.state());
        debug!("Turn of peer #: {:}", self.protocol().next());

        match self.state() {
            RelaySessionState::Initialized => {
                debug!("Relay sessions state is initialized");
            }
            _ => {
                debug!("Relay sessions state is not initialized");
                return Err(STATE_NOT_INITIALIZED);
            }
        }
        // validate the sender in the message (peer_number field) is the peer associated with this address
        let sender = msg.peer_number;
        let peers = self.peers.read().unwrap();
        let peer = peers.get(from);

        // if peer is present and registered
        if let Some(p) = peer {
            if p.registered && p.peer_id == sender {
                // check if it is this peers turn
                if self.protocol().next() == p.peer_id {
                    return Ok(());
                } else {
                    return Err(NOT_YOUR_TURN);
                }
            }
        }
        {
            return Err("Not A peer");
        }
    }
}

impl RelaySession {
    /// Creates a new Relay Session with default (empty) fields
    /// and an Empty state
    pub fn new(capacity: u32) -> RelaySession {
        RelaySession {
            peers: Arc::new(RwLock::new(HashMap::new())),

            active_peers: Arc::new(RwLock::new(0)),

            protocol: Arc::new(RwLock::new(
                relay_server_common::protocol::ProtocolDescriptor::new(0, capacity),
            )),

            state: Arc::new(RwLock::new(RelaySessionState::Empty)),
        }
    }

    /// Inserts a new connection to the session.
    /// the connection is NOT an active peer until it is registered to the session
    /// by sending a register message
    pub fn insert_new_connection(&self, addr: SocketAddr, client: Client) {
        self.peers.write().unwrap().insert(addr, Peer::new(client));
    }

    /// Removes a connection from the peers collection
    fn remove(&self, addr: &SocketAddr) -> Option<Peer> {
        self.peers.write().unwrap().remove(addr)
    }

    /// Send a message from the server to a group of peers
    /// takes each peers 'tx' part of the mpsc channel, and uses it to send the message to the client
    /// this peer represents
    fn multiple_send<E: 'static>(
        &self,
        message: ServerMessage,
        to: &Vec<PeerIdentifier>,
    ) -> Box<dyn Future<Item = (), Error = E>> {
        let peers = self.peers.read().unwrap();
        // For each client, clone its `mpsc::Sender` (because sending consumes the sender) and
        // start sending a clone of `message`. This produces an iterator of Futures.
        //let all_sends = client_map.values().map(|client| client.tx.clone().send(message.clone()));
        let sends = peers
            .values()
            .filter(|peer| {
                let id = &(peer.peer_id as PeerIdentifier);
                to.contains(id) && peer.registered
            })
            .map(|peer| {
                debug!("Sending msg to peer {}: {:?}", peer.peer_id, message);
                peer.client.tx.clone().send(message.clone())
            });

        let send_stream = stream::futures_unordered(sends).then(|_| Ok(()));

        // Convert the stream to a future that runs all the sends and box it up.
        Box::new(send_stream.for_each(|()| Ok(())))
    }

    /// Try to send a server response to a specific address
    pub fn send_response<E: 'static>(
        &self,
        addr: SocketAddr,
        response: ServerMessage,
    ) -> Box<dyn Future<Item = (), Error = E>> /*-> Result<(),()>*/ {
        let peers = self.peers.read().unwrap();
        // For each client, clone its `mpsc::Sender` (b ecause sending consumes the sender) and
        // start sending a clone of `message`. This produces an iterator of Futures.
        //let all_sends = client_map.values().map(|client| client.tx.clone().send(message.clone()));
        if let Some(_peer) = peers.get(&addr) {
            let to = vec![_peer.peer_id as PeerIdentifier];
            self.multiple_send(response, &to)
        } else {
            // TODO: Disconnect peer and end session
            panic!("err")
        }
    }

    /// Try to send a server response to a specific address
    pub fn get_sender_by_address(&self, addr: &SocketAddr) -> Option<mpsc::Sender<ServerMessage>> {
        if let Some(peer) = self.get_peer_by_address(&addr) {
            Some(peer.client.tx.clone())
        } else {
            None
        }
    }

    /// try to send a relay message to the desired peers
    pub fn relay_message<E: 'static>(
        &self,
        from: &SocketAddr,
        msg: RelayMessage,
    ) -> Box<dyn Future<Item = (), Error = E>> {
        let mut server_msg = ServerMessage::new();
        let mut _to = vec![];
        let peer_id = self.peers.read().unwrap().get(from).unwrap().peer_id;
        let can_relay = self.can_relay(from, &msg);
        match can_relay {
            Ok(()) => {
                server_msg.relay_message = Some(msg.clone());
                _to = msg.to;
                self.protocol.write().unwrap().advance_turn();

                debug!(
                    "Sending relay message from peer {:?} to: {:?}",
                    peer_id, _to
                );
            }
            Err(err_msg) => {
                // send an error response to sender
                warn!("Peer {:} can not relay", peer_id);
                server_msg.response = Some(ServerResponse::ErrorResponse(String::from(err_msg)));
                _to = vec![peer_id];
            }
        }
        self.multiple_send(server_msg, &_to)
    }

    /// Register a new peer for the relay session.
    /// Return a vector of register messages to send to all other peers if state is initialized
    pub fn register(
        &self,
        addr: SocketAddr,
        protocol_id: ProtocolIdentifier,
        capacity: u32,
    ) -> Vec<(ServerMessage, mpsc::Sender<ServerMessage>)> {
        self.register_new_peer(addr, protocol_id, capacity);
        // Send message to all
        match self.state() {
            RelaySessionState::Initialized => {
                let peers = self.peers.read().unwrap();
                let sends = peers
                    .iter()
                    .map(|(_addr, peer)| {
                        let mut server_msg = ServerMessage::new();
                        server_msg.response = Some(ServerResponse::Register(peer.peer_id));
                        (server_msg, peer.client.tx.clone())
                    })
                    .collect();
                sends
            }
            _ => vec![],
        }
    }

    pub fn abort(&self, addr: SocketAddr) -> Vec<(ServerMessage, mpsc::Sender<ServerMessage>)> {
        if !(self.state() == RelaySessionState::Initialized) {
            let peer = self.get_peer_by_address(&addr);
            let mut server_msg = ServerMessage::new();
            match peer {
                Some(p) => {
                    server_msg.abort = Some(AbortMessage::new(p.peer_id, self.protocol().id));
                    self.set_state(RelaySessionState::Aborted);
                    let peers = self.peers.read().unwrap();
                    peers
                        .iter()
                        .map(|(_addr, peer)| (server_msg.clone(), peer.client.tx.clone()))
                        .collect()
                }
                None => vec![],
            }
        } else {
            vec![]
        }
    }

    /// handle a closed connection
    /// if it an active peer disconnected - abort the session
    /// otherwise, simply remove the connection of this address from the peers collection
    pub fn connection_closed<E: 'static>(
        &self,
        addr: SocketAddr,
    ) -> Box<dyn Future<Item = (), Error = E>> {
        info!("Connection closed.");
        let mut to = Vec::new();
        // Read registered peers then release the read() lock
        let peers = self
            .peers
            .read()
            .unwrap()
            .values()
            .filter(|p| p.registered)
            .for_each(|p| {
                to.push(p.peer_id);
            });
        drop(peers);

        let mut peer_disconnected = false;
        let mut peer_id = 0;

        let peers = self.peers.write().unwrap();
        // check if the address was a peer
        let peer = peers.get(&addr);
        if peer.is_some() {
            let p = peer.unwrap();
            if !p.registered {
                self.remove(&addr);
                return Box::new(futures::future::ok(()));
            } else {
                peer_id = p.peer_id;
                peer_disconnected = true;
                info!("Aborted from peer #: {:}", peer_id);
            }
        }

        // Release peers write lock
        drop(peers);

        if peer_disconnected {
            info!("Connection closed with a peer. Aborting..");
            let mut server_msg = ServerMessage::new();
            server_msg.abort = Some(AbortMessage::new(peer_id, self.protocol().id));
            self.set_state(RelaySessionState::Aborted);
            return self.multiple_send(server_msg, &to);
        }
        Box::new(futures::future::ok(()))
    }

    /// get a copy of Peer that addr represents
    pub fn get_peer_by_address(&self, addr: &SocketAddr) -> Option<Peer> {
        match self.peers.read().unwrap().get(addr) {
            Some(p) => match p.registered {
                true => Some(p.clone()),
                false => None,
            },
            None => None,
        }
    }

    // Return the current state of the relay session
    pub fn state(&self) -> RelaySessionState {
        self.state.read().unwrap().clone()
    }

    // Set the current relay session state to a new state
    pub fn set_state(&self, new_state: RelaySessionState) {
        *self.state.write().unwrap() = new_state;
    }

    pub fn protocol(&self) -> ProtocolDescriptor {
        self.protocol.read().unwrap().clone()
    }

    pub fn set_protocol(&self, protocol: ProtocolDescriptor) {
        *self.protocol.write().unwrap() = protocol;
    }
}

#[cfg(test)]
mod tests {
    use super::Client;
    use super::RelaySession;
    use super::RelaySessionState;
    use futures::sync::mpsc;
    use relay_server_common::protocol::ProtocolDescriptor;
    use relay_server_common::{ProtocolIdentifier, ServerMessageType};
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_add_peer() {
        let protocol_id: ProtocolIdentifier = 1;
        let capacity: u32 = 1;
        let rs = RelaySession::new(capacity);
        let client_addr: SocketAddr = "127.0.0.1:8081".parse().unwrap();
        let (tx, _) = mpsc::channel(0);
        rs.insert_new_connection(client_addr.clone(), Client::new(tx));

        let peer_num = rs.register_new_peer(client_addr, protocol_id, capacity);
        assert_eq!(peer_num, Some(1));
    }

    #[test]
    fn test_add_multi_peers() {
        let protocol_id: ProtocolIdentifier = 1;
        let capacity: u32 = 5;
        let rs = RelaySession::new(capacity);

        let mut peer_num: u32 = 0;
        for i in 0..capacity {
            let client_addr: SocketAddr = format!("127.0.0.1:808{}", i).parse().unwrap();
            let (tx, _) = mpsc::channel(0);
            rs.insert_new_connection(client_addr.clone(), Client::new(tx));
            peer_num = rs
                .register_new_peer(client_addr, protocol_id, capacity)
                .expect("Unable to register");
        }

        assert_eq!(peer_num, capacity);
    }

    #[test]
    fn test_add_multi_peers_in_parallel() {
        let mut children = vec![];

        let protocol_id: ProtocolIdentifier = 1;
        let capacity: u32 = 50;
        let rs = Arc::new(RelaySession::new(capacity));

        for i in 0..capacity {
            let rs_inner = Arc::clone(&rs);

            let client_addr: SocketAddr = format!("127.0.0.1:80{}", 30 + i).parse().unwrap();
            let (tx, _) = mpsc::channel(0);
            children.push(thread::spawn(move || {
                rs_inner.insert_new_connection(client_addr.clone(), Client::new(tx));
                rs_inner
                    .register_new_peer(client_addr, protocol_id, capacity)
                    .expect("Unable to register");
            }));
        }

        for child in children {
            let _ = child.join();
        }
        let rs_inner = Arc::clone(&rs);
        let number_of_active_peers = rs_inner.get_number_of_active_peers();
        assert_eq!(number_of_active_peers, capacity);
    }

    #[test]
    fn test_can_register_protocol_valid() {
        let client_addr: SocketAddr = format!("127.0.0.1:8081").parse().unwrap();
        let protocol_id: ProtocolIdentifier = 1 as ProtocolIdentifier;
        let capacity: u32 = 5;
        let protocol_descriptor = ProtocolDescriptor::new(protocol_id, capacity);
        let rs = RelaySession::new(capacity);
        let (tx, _) = mpsc::channel(0);
        rs.insert_new_connection(client_addr.clone(), Client::new(tx));
        assert!(rs.can_register(&client_addr, protocol_descriptor))
    }

    #[test]
    fn test_can_register_protocol_invalid() {
        let client_addr: SocketAddr = format!("127.0.0.1:8081").parse().unwrap();
        let protocol_id: ProtocolIdentifier = 100 as ProtocolIdentifier;
        let capacity: u32 = 5;
        let protocol_descriptor = ProtocolDescriptor::new(protocol_id, capacity);
        let rs = RelaySession::new(capacity);
        let (tx, _) = mpsc::channel(0);
        rs.insert_new_connection(client_addr.clone(), Client::new(tx));
        assert!(!rs.can_register(&client_addr, protocol_descriptor))
    }

    #[test]
    fn test_can_register_no_connection() {
        let client_addr: SocketAddr = format!("127.0.0.1:8081").parse().unwrap();
        let protocol_id: ProtocolIdentifier = 1 as ProtocolIdentifier;
        let capacity: u32 = 5;
        let protocol_descriptor = ProtocolDescriptor::new(protocol_id, capacity);
        let rs = RelaySession::new(capacity);
        assert!(!rs.can_register(&client_addr, protocol_descriptor))
    }

    #[test]
    fn test_can_register_already_connected() {
        let client_addr: SocketAddr = format!("127.0.0.1:8081").parse().unwrap();
        let protocol_id: ProtocolIdentifier = 1 as ProtocolIdentifier;
        let capacity: u32 = 5;
        let protocol_descriptor = ProtocolDescriptor::new(protocol_id, capacity);
        let rs = RelaySession::new(capacity);
        rs.register(client_addr, protocol_id, capacity);
        assert!(!rs.can_register(&client_addr, protocol_descriptor))
    }

    /////////////////////////// test register ///////////////////////////////////
    #[test]
    fn test_register_state() {
        let protocol_id: ProtocolIdentifier = 1;
        let capacity: u32 = 4;
        let rs = RelaySession::new(capacity);

        // State is empty at fist
        assert_eq!(RelaySessionState::Empty, rs.state());
        for i in 0..capacity - 1 {
            let client_addr: SocketAddr = format!("127.0.0.1:808{}", i).parse().unwrap();
            let (tx, _) = mpsc::channel(0);
            rs.insert_new_connection(client_addr.clone(), Client::new(tx));
            rs.register(client_addr, protocol_id, capacity);
            // State is not initialized when not all are connected
            assert_eq!(RelaySessionState::Uninitialized, rs.state());
        }
        let client_addr: SocketAddr = format!("127.0.0.1:808{}", capacity - 1).parse().unwrap();
        let (tx, _) = mpsc::channel(0);
        rs.insert_new_connection(client_addr.clone(), Client::new(tx));
        let messages = rs.register(client_addr, protocol_id, capacity);
        // Once all are connected, state should initialize
        assert_eq!(RelaySessionState::Initialized, rs.state());

        messages
            .iter()
            .for_each(|(msg, _)| assert_eq!(msg.msg_type(), ServerMessageType::Response));
    }

    /////////////////////////// test register ///////////////////////////////////
}
