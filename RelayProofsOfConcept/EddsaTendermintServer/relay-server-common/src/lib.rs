use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::vec::Vec;
use tokio_jsoncodec::Codec as JsonCodec;

pub mod common;
pub mod protocol;

pub type ProtocolIdentifier = u32;
pub type PeerIdentifier = u32;
pub type MessagePayload = String;

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
    pub messages: HashMap<u32, HashMap<u32, ClientMessage>>,
}

impl StoredMessages {
    pub fn new() -> StoredMessages {
        StoredMessages {
            messages: HashMap::new(),
        }
    }

    pub fn update(&mut self, round: u32, party: u32, msg: ClientMessage) {
        self.messages.entry(round).or_insert(HashMap::new());
        match self.messages.get_mut(&round) {
            Some(messages) => {
                messages.insert(party, msg.clone());
            }
            _ => (),
        }
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

    pub fn register(
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
        println!("{:?}", stored_messages);
    }
}
