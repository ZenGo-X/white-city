use mmpc_server_common::{MessagePayload, PeerIdentifier};

pub const MAX_CLIENTS: usize = 12;

pub trait Peer {
    fn new(capacity: u32, message: Vec<u8>, index: u32) -> Self;
    fn zero_step(&mut self, peer_id: PeerIdentifier) -> Option<MessagePayload>;
    fn current_step(&self) -> u32;
    fn capacity(&self) -> u32;
    fn peer_id(&self) -> PeerIdentifier;
    fn set_peer_id(&mut self, peer_id: PeerIdentifier);
    fn do_step(&mut self);
    fn update_data(&mut self, from: PeerIdentifier, payload: MessagePayload);
    fn get_next_item(&mut self) -> Option<MessagePayload>;
    fn finalize(&mut self) -> Result<(), &'static str>;
    fn is_done(&mut self) -> bool;
}

pub struct ProtocolDataManager<T: Peer> {
    pub data_holder: T, // will be filled when initializing, and on each new step
    pub client_data: Option<MessagePayload>, // new data calculated by this peer at the beginning of a step (that needs to be sent to other peers)
    pub new_client_data: bool,
}

impl<T: Peer> ProtocolDataManager<T> {
    pub fn new(capacity: u32, message: Vec<u8>, index: u32) -> ProtocolDataManager<T>
    where
        T: Peer,
    {
        ProtocolDataManager {
            data_holder: Peer::new(capacity, message, index),
            client_data: None,
            new_client_data: false,
        }
    }

    /// set manager with the initial values that a local peer holds at the beginning of
    /// the protocol session
    /// return: first message
    pub fn initialize_data(&mut self, peer_id: PeerIdentifier) -> Option<MessagePayload> {
        self.data_holder.set_peer_id(peer_id);
        let zero_step_data = self.data_holder.zero_step(peer_id);
        self.client_data = zero_step_data;
        return self.client_data.clone();
    }

    /// Get the next message this client needs to send
    pub fn get_next_message(
        &mut self,
        from: PeerIdentifier,
        payload: MessagePayload,
    ) -> Option<MessagePayload> {
        self.data_holder.update_data(from, payload);
        self.data_holder.do_step();
        self.data_holder.get_next_item()
    }
}
