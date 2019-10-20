use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::vec::Vec;
use tokio_jsoncodec::Codec as JsonCodec;

pub mod common;
pub mod protocol;

pub type ProtocolIdentifier = u32;
pub type PeerIdentifier = u32;
pub type MessagePayload = String;

const MAX_CLIENTS: u32 = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayMessage {
    pub peer_number: PeerIdentifier,
    pub protocol_id: ProtocolIdentifier,
    pub from: SocketAddr,
    pub to: Vec<PeerIdentifier>,
    pub message: MessagePayload,
}

impl RelayMessage {
    pub fn new(
        peer_number: PeerIdentifier,
        protocol_id: ProtocolIdentifier,
        from: SocketAddr,
    ) -> RelayMessage {
        RelayMessage {
            peer_number,
            protocol_id,
            from,
            to: Vec::new(),
            message: String::from(""),
        }
    }

    pub fn set_message_params<S: Into<String>>(&mut self, to: Vec<PeerIdentifier>, message: S) {
        //self.round = round_number;
        self.to = to;
        self.message = message.into();
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ServerResponse {
    // Register response containing peer number
    Register(PeerIdentifier),

    // Error message
    ErrorResponse(String),

    // No response
    NoResponse,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct AbortMessage {
    pub peer_number: PeerIdentifier,
    pub protocol_id: ProtocolIdentifier,
}

impl AbortMessage {
    pub fn new(peer_number: PeerIdentifier, protocol_id: ProtocolIdentifier) -> AbortMessage {
        AbortMessage {
            peer_number,
            protocol_id,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RegisterMessage {
    pub addr: SocketAddr,

    pub protocol_id: ProtocolIdentifier,

    pub capacity: u32,

    pub index: i32,
}

#[derive(Debug, PartialEq)]
pub enum ServerMessageType {
    Response,
    Abort,
    RelayMessage,
    Undefined,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct ServerMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abort: Option<AbortMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<ServerResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub relay_message: Option<RelayMessage>,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct MissingMessagesRequest {
    pub round: u32,
    pub missing_clients: Vec<u32>,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct MissingMessagesReply {
    pub missing_messages: BTreeMap<u32, ClientMessage>,
}

impl ServerMessage {
    pub fn new() -> ServerMessage {
        ServerMessage {
            response: None,

            abort: None,

            relay_message: None,
        }
    }

    pub fn msg_type(&self) -> ServerMessageType {
        if self.response.is_some() {
            return ServerMessageType::Response;
        }
        if self.relay_message.is_some() {
            return ServerMessageType::RelayMessage;
        }
        if self.abort.is_some() {
            return ServerMessageType::Abort;
        }
        return ServerMessageType::Undefined;
    }
}

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct StoredMessages {
    pub messages: BTreeMap<u32, BTreeMap<u32, ClientMessage>>,
}

impl StoredMessages {
    pub fn new() -> StoredMessages {
        StoredMessages {
            messages: BTreeMap::new(),
        }
    }

    // Insert a new ClientMessage for a given round, and a given party
    pub fn update(&mut self, round: u32, party: u32, msg: ClientMessage) {
        self.messages.entry(round).or_insert(BTreeMap::new());
        match self.messages.get_mut(&round) {
            Some(messages) => {
                messages.insert(party, msg.clone());
            }
            _ => (),
        }
    }

    // Return the current number of stored messages
    pub fn get_number_messages(&self, round: u32) -> usize {
        match self.messages.get(&round) {
            Some(messages) => return messages.keys().len(),
            None => 0,
        }
    }

    // Returns the messages of the current round as client messages format,
    // or an empty vector if no messages are stored for the round
    pub fn get_messages_vector_client_message(&self, round: u32) -> Vec<ClientMessage> {
        match self.messages.get(&round) {
            Some(round_messages) => {
                let mut response_vec = Vec::new();
                for (_client_idx, msg) in round_messages.iter() {
                    response_vec.push(msg.clone());
                }
                return response_vec;
            }
            None => return Vec::new(),
        }
    }

    // Returns the messages of the current round as client messages format,
    // or an empty hashmap if no messages are stored for the round
    pub fn get_messages_map_client_message(&self, round: u32) -> BTreeMap<u32, ClientMessage> {
        match self.messages.get(&round) {
            Some(round_messages) => {
                let mut response = BTreeMap::new();
                // Only return a response on the first MAX clients
                let mut max_counter = 0;
                for (client_idx, msg) in round_messages.iter() {
                    let idx = *client_idx as u32;
                    response.insert(idx, msg.clone());
                    max_counter += 1;
                    if max_counter > MAX_CLIENTS {
                        break;
                    }
                }
                return response;
            }
            None => return BTreeMap::new(),
        }
    }

    // Returns the messages of the current round as client messages format,
    // or an empty hashmap if no messages are stored for the round
    pub fn get_messages_map_from_vector(
        &self,
        round: u32,
        missing_clients: &[u32],
    ) -> BTreeMap<u32, ClientMessage> {
        match self.messages.get(&round) {
            Some(round_messages) => {
                // TODO: Rewrite with filter and iterator
                let mut response_vec = BTreeMap::new();
                for (client_idx, msg) in round_messages.iter() {
                    let idx = *client_idx as u32;
                    if missing_clients.contains(&idx) {
                        response_vec.insert(idx, msg.clone());
                    }
                }
                return response_vec;
            }
            None => return BTreeMap::new(),
        }
    }

    // Return a vector of all clients whos messages are not yet stored for a given round
    pub fn get_missing_clients_vector(&self, round: u32, capacity: u32) -> Vec<u32> {
        let return_vec;
        match self.messages.get(&round) {
            Some(keys) => {
                return_vec = (1..capacity + 1)
                    .filter(|x| !keys.contains_key(x))
                    .collect();
            }
            None => {
                return_vec = (1..capacity + 1).collect();
            }
        }
        return_vec
    }
}

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct ClientMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub register: Option<RegisterMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub abort: Option<AbortMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub relay_message: Option<RelayMessage>,
}

impl ClientMessage {
    pub fn new() -> ClientMessage {
        ClientMessage {
            register: None,

            abort: None,

            relay_message: None,
        }
    }

    pub fn set_register(
        &mut self,
        addr: SocketAddr,
        protocol_id: ProtocolIdentifier,
        capacity: u32,
        index: i32,
    ) {
        self.register = Some(RegisterMessage {
            addr,
            protocol_id,
            capacity,
            index,
        });
    }

    pub fn is_empty(&self) -> bool {
        self.relay_message.is_none() && self.abort.is_none() && self.register.is_none()
    }

    pub fn are_equal_payloads(&self, msg: &ClientMessage) -> bool {
        if self.register.is_some() && msg.register.is_some() {
            return true;
        } else if self.relay_message.is_some() && msg.relay_message.is_some() {
            let self_message = self.relay_message.clone().unwrap().message;
            let message = msg.relay_message.clone().unwrap().message;
            return self_message == message;
        } else if self.abort.is_some() && msg.abort.is_some() {
            return true;
        }
        false
    }

    pub fn msg_type(&self) -> ClientMessageType {
        if self.register.is_some() {
            return ClientMessageType::Register;
        }
        if self.relay_message.is_some() {
            return ClientMessageType::RelayMessage;
        }
        if self.abort.is_some() {
            return ClientMessageType::Abort;
        }
        return ClientMessageType::Undefined;
    }
}

#[derive(Debug)]
pub enum ClientMessageType {
    Register,
    Abort,
    RelayMessage,
    Undefined,
    Test,
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct RegisterResponse {
    peer_number: PeerIdentifier,
}

// in: clientMessage out:serverMessage
pub type ServerToClientCodec = JsonCodec<ClientMessage, ServerMessage>;
pub type ClientToServerCodec = JsonCodec<ServerMessage, ClientMessage>;

#[cfg(test)]
mod tests {
    use super::ClientMessage;
    use super::StoredMessages;

    #[test]
    fn test_stored_messages() {
        let mut stored_messages = StoredMessages::new();
        stored_messages.update(1, 3, ClientMessage::new());
        stored_messages.update(1, 2, ClientMessage::new());
    }

    #[test]
    fn test_get_number_messages() {
        let mut stored_messages = StoredMessages::new();
        let round = 1;
        stored_messages.update(round, 3, ClientMessage::new());
        stored_messages.update(round, 2, ClientMessage::new());
        assert_eq!(stored_messages.get_number_messages(round), 2);
        // Test no messages for a round where none where inserted
        assert_eq!(stored_messages.get_number_messages(3), 0);
    }

    #[test]
    fn test_get_missing_clients_vector() {
        let mut stored_messages = StoredMessages::new();
        let round = 1;
        let capacity = 4;
        stored_messages.update(round, 3, ClientMessage::new());
        stored_messages.update(round, 2, ClientMessage::new());
        assert_eq!(
            stored_messages.get_missing_clients_vector(round, capacity),
            [1, 4]
        );
        // Test an empty round
        assert_eq!(
            stored_messages.get_missing_clients_vector(round + 1, capacity),
            [1, 2, 3, 4]
        );
    }

    #[test]
    fn test_get_messages_map_client_message() {
        let mut stored_messages = StoredMessages::new();
        let round = 1;
        stored_messages.update(round, 3, ClientMessage::new());
        stored_messages.update(round, 2, ClientMessage::new());
        let mut i: u32 = 2;
        // Assert all messages are stored in order of round and client
        for (idx, _) in stored_messages.get_messages_map_client_message(round) {
            assert_eq!(i, idx);
            i += 1;
        }
        // Assert sorted order for non sequential client messages
        let mut stored_messages = StoredMessages::new();
        stored_messages.update(round, 4, ClientMessage::new());
        stored_messages.update(round, 2, ClientMessage::new());
        let mut i: u32 = 2;
        for (idx, _) in stored_messages.get_messages_map_client_message(round) {
            assert_eq!(i, idx);
            i += 2;
        }
        // Test for more that MAX clients
        let mut stored_messages = StoredMessages::new();
        stored_messages.update(round, 1, ClientMessage::new());
        stored_messages.update(round, 2, ClientMessage::new());
        stored_messages.update(round, 3, ClientMessage::new());
        stored_messages.update(round, 4, ClientMessage::new());
        stored_messages.update(round, 5, ClientMessage::new());
        stored_messages.update(round, 6, ClientMessage::new());
        stored_messages.update(round, 7, ClientMessage::new());
        stored_messages.update(round, 8, ClientMessage::new());
        stored_messages.update(round, 9, ClientMessage::new());
        stored_messages.update(round, 10, ClientMessage::new());
        stored_messages.update(round, 11, ClientMessage::new());
        stored_messages.update(round, 12, ClientMessage::new());
        stored_messages.update(round, 13, ClientMessage::new());
        stored_messages.update(round, 14, ClientMessage::new());
        assert_eq!(
            stored_messages
                .get_messages_map_client_message(round)
                .into_iter()
                .len(),
            13
        );
    }

    #[test]
    fn test_get_messages_from_vector() {
        let mut stored_messages = StoredMessages::new();
        let round = 1;
        stored_messages.update(round, 3, ClientMessage::new());
        stored_messages.update(round, 2, ClientMessage::new());
        // Assert all messages are stored in order of round and client
        println!(
            "Stored {:?}",
            stored_messages.get_messages_map_from_vector(round, &[2])
        );
    }
}
