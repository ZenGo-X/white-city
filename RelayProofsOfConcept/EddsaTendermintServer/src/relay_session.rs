use log::{debug, info, warn};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use mmpc_server_common::{ClientMessage, StoredMessages};
use mmpc_server_common::{PeerIdentifier, ProtocolIdentifier, RelayMessage};

use mmpc_server_common::protocol::ProtocolDescriptor;

#[derive(Clone, Debug)]
pub struct Peer {
    pub peer_id: PeerIdentifier,
    pub addr: SocketAddr,
    pub registered: bool,
}

impl Peer {
    pub fn new(addr: SocketAddr) -> Peer {
        Peer {
            peer_id: 0,
            addr: addr,
            registered: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelaySessionState {
    Empty,

    Uninitialized,

    Initialized,
}

#[derive(Debug, Clone)]
pub struct RelaySession {
    peers: Arc<RwLock<HashMap<SocketAddr, Peer>>>,

    active_peers: Arc<RwLock<u32>>,

    protocol: Arc<RwLock<ProtocolDescriptor>>,

    state: Arc<RwLock<RelaySessionState>>,

    round: Arc<RwLock<u32>>,

    stored_messages: Arc<RwLock<StoredMessages>>,
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
        index: i32,
    ) -> Option<u32> {
        let _addr = &addr;
        let number_of_active_peers = self.get_number_of_active_peers();

        let protocol_descriptor = ProtocolDescriptor::new(protocol_id, capacity);
        debug!("-----------------PEERS: {:?}---------------", self.peers);
        if self.can_register(_addr, protocol_descriptor) {
            let mut peer = Peer::new(addr);
            peer.registered = true;
            peer.peer_id = number_of_active_peers + 1;

            self.peers.write().unwrap().insert(addr, peer);

            // activate this connection as a peer
            // if needed, set the ProtocolDescriptor for this sessuib
            // and change the state
            let state = self.state();
            if let RelaySessionState::Empty = state {
                self.set_protocol(ProtocolDescriptor::new(protocol_id, capacity));
                info!("Relay session state is now Uninitialized");
                self.set_state(RelaySessionState::Uninitialized);
            }
            //if self.protocol.clone().into_inner().capacity == number_of_active_peers + 1 {
            if self.protocol().capacity == number_of_active_peers + 1 {
                info!("Relay session state is now Initialized");
                self.set_state(RelaySessionState::Initialized);
            }
            info!("Registered peer {}", number_of_active_peers + 1);
            if index == -1 {
                Some(number_of_active_peers + 1) //peer_id
            } else {
                Some(index as u32) //peer_id
            }
        } else {
            warn!("Unable to register {:}", addr); // error
            None
        }
    }

    /// Checks if it is possible for this address
    /// to register as a peer in this session
    pub fn can_register(&self, _addr: &SocketAddr, protocol: ProtocolDescriptor) -> bool {
        match self.state() {
            // if this is the first peer to register
            // check that the protocol is valid
            RelaySessionState::Empty => {
                debug!("Checking if protocol description is valid");
                if !mmpc_server_common::protocol::is_valid_protocol(&protocol) {
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
        true
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
                mmpc_server_common::protocol::ProtocolDescriptor::new(0, capacity),
            )),

            state: Arc::new(RwLock::new(RelaySessionState::Empty)),

            round: Arc::new(RwLock::new(0)),

            stored_messages: Arc::new(RwLock::new(StoredMessages::new())),
        }
    }

    /// Check if this relay message sent from the given SocketAddr
    /// and is valid to send to rest of the peers
    pub fn can_relay(&self, _from: &SocketAddr, msg: &RelayMessage) -> Result<(), &'static str> {
        debug!("Checking if {:} can relay", msg.peer_number);
        debug!("Server state: {:?}", self.state());
        debug!("Turn of peer #: {:}", self.protocol().next());

        // TODO: Add some checks of what messages can be stored

        return Ok(());
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

    pub fn round(&self) -> u32 {
        self.round.read().unwrap().clone()
    }

    pub fn update_stored_messages(&mut self, round: u32, party: u32, msg: ClientMessage) {
        self.stored_messages
            .write()
            .unwrap()
            .update(round, party, msg);
    }

    pub fn stored_messages(&self) -> StoredMessages {
        self.stored_messages.read().unwrap().clone()
    }

    pub fn try_increase_round(&self, capacity: u32) {
        if self
            .stored_messages
            .read()
            .unwrap()
            .get_number_messages(self.round.read().unwrap().clone())
            == capacity as usize
        {
            *self.round.write().unwrap() += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RelaySession;
    use super::RelaySessionState;

    use mmpc_server_common::protocol::ProtocolDescriptor;
    use mmpc_server_common::ProtocolIdentifier;

    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_add_peer() {
        let protocol_id: ProtocolIdentifier = 1;
        let capacity: u32 = 1;
        let rs = RelaySession::new(capacity);
        let client_addr: SocketAddr = format!("127.0.0.1:808{}", 0).parse().unwrap();

        let peer_num = rs.register_new_peer(client_addr, protocol_id, capacity, 0);
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
            peer_num = rs
                .register_new_peer(client_addr, protocol_id, capacity, 0)
                .expect("Unable to register");
            println!("Peer number is {}", peer_num);
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
            children.push(thread::spawn(move || {
                rs_inner
                    .register_new_peer(client_addr, protocol_id, capacity, -1)
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
        let client_addr: SocketAddr = "127.0.0.1:8081".to_string().parse().unwrap();
        let protocol_id: ProtocolIdentifier = 1 as ProtocolIdentifier;
        let capacity: u32 = 5;
        let protocol_descriptor = ProtocolDescriptor::new(protocol_id, capacity);
        let rs = RelaySession::new(capacity);
        assert!(rs.can_register(&client_addr, protocol_descriptor))
    }

    #[test]
    fn test_can_register_protocol_invalid() {
        let client_addr: SocketAddr = "127.0.0.1:8081".to_string().parse().unwrap();
        let protocol_id: ProtocolIdentifier = 100 as ProtocolIdentifier;
        let capacity: u32 = 5;
        let protocol_descriptor = ProtocolDescriptor::new(protocol_id, capacity);
        let rs = RelaySession::new(capacity);
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
            rs.register_new_peer(client_addr, protocol_id, capacity, -1);
            // State is not initialized when not all are connected
            assert_eq!(RelaySessionState::Uninitialized, rs.state());
        }
        let client_addr: SocketAddr = format!("127.0.0.1:808{}", capacity - 1).parse().unwrap();
        let messages = rs.register_new_peer(client_addr, protocol_id, capacity, -1);
        // Once all are connected, state should initialize
        assert_eq!(RelaySessionState::Initialized, rs.state());
    }
}
