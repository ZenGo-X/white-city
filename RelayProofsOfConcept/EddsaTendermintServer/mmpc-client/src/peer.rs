use mmpc_server_common::{MessagePayload, PeerIdentifier};

pub const MAX_CLIENTS: usize = 12;

#[derive(Debug)]
pub enum MessagePayloadType {
    /// Types of expected relay messages
    /// for step 0 we expect PUBLIC_KEY_MESSAGE
    /// for step 1 we expect Commitment
    /// for step 2 we expect RMessage
    /// for step 3 we expect Signature
    PublicKey(String),
    // Commitment(String),
    // RMessage(String),
    // Signature(String),
}

pub trait Peer {
    fn new(capacity: u32, message: Vec<u8>, index: u32) -> Self;
    fn zero_step(&mut self, peer_id: PeerIdentifier) -> Option<MessagePayload>;
    fn current_step(&self) -> u32;
    fn do_step(&mut self);
    fn update_data(&mut self, from: PeerIdentifier, payload: MessagePayload);
    fn get_next_item(&mut self) -> Option<MessagePayload>;
    fn finalize(&mut self) -> Result<(), &'static str>;
    fn is_done(&mut self) -> bool;
}
