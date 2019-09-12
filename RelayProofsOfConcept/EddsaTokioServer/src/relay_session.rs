use futures::sync::mpsc;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use relay_server_common::{
    AbortMessage, PeerIdentifier, ProtocolIdentifier, RelayMessage, ServerMessage, ServerResponse,
};

use relay_server_common::common::{NOT_A_PEER, NOT_YOUR_TURN, STATE_NOT_INITIALIZED};

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

    /// Check if this relay message sent from the given SocketAddr
    /// and is valid to send to rest of the peers
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
        let peer = self.get_peer_by_address(from);

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
        return Err(NOT_A_PEER);
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

    /// Try reutrn a Sender of a specific peer by its address.
    /// The Sender can be used to send ServerMessages to the server
    pub fn get_sender_by_address(&self, addr: &SocketAddr) -> Option<mpsc::Sender<ServerMessage>> {
        if let Some(peer) = self.get_peer_by_address(&addr) {
            Some(peer.client.tx.clone())
        } else {
            None
        }
    }

    /// Receives the sender's address and a message
    /// If the message can be relayed, returns a vector of tupltes,
    /// with the message as the first member, and a Sender to recipient as the second
    pub fn relay_message(
        &self,
        from: &SocketAddr,
        msg: RelayMessage,
    ) -> Vec<(ServerMessage, mpsc::Sender<ServerMessage>)> {
        let mut server_msg = ServerMessage::new();
        let sender = self.get_peer_by_address(from).unwrap();
        let sender_id = sender.peer_id;
        let can_relay = self.can_relay(from, &msg);
        match can_relay {
            Ok(()) => {
                server_msg.relay_message = Some(msg.clone());
                let peers = self.peers.read().unwrap();
                let messages_to_send = peers
                    .values()
                    .filter(|peer| {
                        let id = &(peer.peer_id as PeerIdentifier);
                        msg.to.contains(id) && peer.registered
                    })
                    .map(|peer| (server_msg.clone(), peer.client.tx.clone()))
                    .collect();
                self.protocol.write().unwrap().advance_turn();

                debug!(
                    "Sending relay message from peer {:?} to: {:?}",
                    sender_id, msg.to
                );
                messages_to_send
            }
            Err(err_msg) => {
                // send an error response to sender
                warn!("Peer {:} can not relay", sender_id);
                server_msg.response = Some(ServerResponse::ErrorResponse(String::from(err_msg)));
                vec![(server_msg, sender.client.tx.clone())]
            }
        }
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

    // Abort the current relay session
    // Return an abort message to all connected peers
    pub fn abort(&self, addr: SocketAddr) -> Vec<(ServerMessage, mpsc::Sender<ServerMessage>)> {
        warn!("Received abort, sending abort messages to all");
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

    use relay_server_common::common::{NOT_A_PEER, NOT_YOUR_TURN, STATE_NOT_INITIALIZED};
    use relay_server_common::protocol::ProtocolDescriptor;
    use relay_server_common::{
        ClientMessage, PeerIdentifier, ProtocolIdentifier, RelayMessage, ServerMessageType,
    };

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

        // State is empty at first
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

    /////////////////////////// test abort ///////////////////////////////////
    #[test]
    fn test_abort() {
        let protocol_id: ProtocolIdentifier = 1;
        let capacity: u32 = 4;
        let rs = RelaySession::new(capacity);

        // State is empty at first
        for i in 0..capacity - 1 {
            let client_addr: SocketAddr = format!("127.0.0.1:808{}", i).parse().unwrap();
            let (tx, _) = mpsc::channel(0);
            rs.insert_new_connection(client_addr.clone(), Client::new(tx));
            rs.register(client_addr, protocol_id, capacity);
            // State is not initialized when not all are connected
            assert_eq!(RelaySessionState::Uninitialized, rs.state());
        }
        let client_addr: SocketAddr = format!("127.0.0.1:808{}", 1).parse().unwrap();

        let messages = rs.abort(client_addr);
        // Once all are connected, state should initialize
        assert_eq!(RelaySessionState::Aborted, rs.state());

        messages
            .iter()
            .for_each(|(msg, _)| assert_eq!(msg.msg_type(), ServerMessageType::Abort));
    }

    fn prepare_relay_message(
        peer_id: PeerIdentifier,
        protocol_id: ProtocolIdentifier,
        send_to: &Vec<PeerIdentifier>,
    ) -> ClientMessage {
        let mut client_message = ClientMessage::new();
        let mut relay_message = RelayMessage::new(peer_id, protocol_id);
        let to: Vec<_> = send_to.clone();

        relay_message.set_message_params(to, format!("test"));
        client_message.relay_message = Some(relay_message.clone());
        client_message.clone()
    }

    /////////////////////////// test can_relay ///////////////////////////////////
    #[test]
    fn test_can_relay() {
        let protocol_id: ProtocolIdentifier = 1;
        let capacity: u32 = 4;
        let rs = RelaySession::new(capacity);

        // Add all but the last peer to the session
        for i in 0..capacity - 1 {
            let client_addr: SocketAddr = format!("127.0.0.1:808{}", i + 1).parse().unwrap();
            let (tx, _) = mpsc::channel(0);
            rs.insert_new_connection(client_addr.clone(), Client::new(tx));
            rs.register(client_addr, protocol_id, capacity);
            let msg = prepare_relay_message(i, protocol_id, &vec![]);
            assert_eq!(
                Err(STATE_NOT_INITIALIZED),
                rs.can_relay(&client_addr, &msg.relay_message.unwrap())
            );
        }
        // Add the last peer to the session
        let client_addr: SocketAddr = format!("127.0.0.1:808{}", capacity).parse().unwrap();
        let (tx, _) = mpsc::channel(0);
        rs.insert_new_connection(client_addr.clone(), Client::new(tx));
        rs.register(client_addr, protocol_id, capacity);
        // Try to relay when not your turn
        let msg = prepare_relay_message(capacity, protocol_id, &vec![]);
        //rs.can_relay(&client_addr, &msg.relay_message.unwrap());
        assert_eq!(
            Err(NOT_YOUR_TURN),
            rs.can_relay(&client_addr, &msg.relay_message.unwrap())
        );
        let client_addr: SocketAddr = format!("127.0.0.1:808{}", capacity + 1).parse().unwrap();
        let msg = prepare_relay_message(capacity, protocol_id, &vec![]);
        assert_eq!(
            Err(NOT_A_PEER),
            rs.can_relay(&client_addr, &msg.relay_message.unwrap())
        );
        let client_addr: SocketAddr = format!("127.0.0.1:808{}", 1).parse().unwrap();
        let msg = prepare_relay_message(1, protocol_id, &vec![]);
        assert_eq!(
            Ok(()),
            rs.can_relay(&client_addr, &msg.relay_message.unwrap())
        );
    }

    /////////////////////////// test rellay_message   ///////////////////////////////////
    #[test]
    fn test_relay_message() {
        let protocol_id: ProtocolIdentifier = 1;
        let capacity: u32 = 4;
        let rs = RelaySession::new(capacity);

        // Add all peers to the session
        for i in 0..capacity {
            let client_addr: SocketAddr = format!("127.0.0.1:808{}", i).parse().unwrap();
            let (tx, _) = mpsc::channel(0);
            rs.insert_new_connection(client_addr.clone(), Client::new(tx));
            rs.register(client_addr, protocol_id, capacity);
        }
        let client_num = 1;
        let msg = prepare_relay_message(client_num, protocol_id, &vec![2, 3, 4]);
        let client_addr: SocketAddr = format!("127.0.0.1:808{}", client_num - 1).parse().unwrap();
        let messages_to_send = rs.relay_message(&client_addr, msg.relay_message.unwrap());
        assert_eq!(messages_to_send.len(), 3);
    }

}
