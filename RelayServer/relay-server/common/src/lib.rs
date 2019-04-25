#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate tokio_core;
extern crate byteorder;

use std::vec::Vec;
use serde::{Serialize, Deserialize};

mod codec;
pub mod protocol;
pub mod common;

pub type ProtocolIdentifier = u32;
pub type PeerIdentifier = u32;
pub type MessagePayload = String;
//pub type MessagePayload = serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayMessage {
    pub peer_number: PeerIdentifier, // from
    pub protocol_id: ProtocolIdentifier,
    //pub round: u32,
    pub to: Vec<PeerIdentifier>,
    pub message: MessagePayload
}

impl RelayMessage {
    pub fn new(peer_number: PeerIdentifier, protocol_id: ProtocolIdentifier) -> RelayMessage{
        let s = r#"{}"#;
        RelayMessage {
            peer_number,
            protocol_id,
        //    round: 0,
            to: Vec::new(),
            message: String::from(""),
        }
    }

    pub fn set_message_params<S: Into<String>>(
        &mut self,
      //  round_number: u32,
        to: Vec<PeerIdentifier>,
        message: S
    )
    {
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

    // General Response
//    GeneralResponse(String),

    // No response
    NoResponse
}

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct AbortMessage {
    pub peer_number: PeerIdentifier,
    pub protocol_id: ProtocolIdentifier,
}

impl AbortMessage{
    pub fn new(peer_number: PeerIdentifier, protocol_id: ProtocolIdentifier) -> AbortMessage {
        AbortMessage{
            peer_number,
            protocol_id,
        }
    }
}

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct RegisterMessage {
    pub protocol_id: ProtocolIdentifier,

    pub capacity: u32,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct ServerMessage {

    #[serde(skip_serializing_if = "Option::is_none")]
    pub abort: Option<AbortMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<ServerResponse>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub relay_message: Option<RelayMessage>
}


impl ServerMessage {
    pub fn new() -> ServerMessage {
        ServerMessage {
            response: None,

            abort: None,

            relay_message: None

        }
    }

}

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub struct ClientMessage {
   // #[serde(skip_serializing_if = "Option::is_none")]
    pub register: Option<RegisterMessage>,

    //#[serde(skip_serializing_if = "Option::is_none")]
    pub abort: Option<AbortMessage>,

    //#[serde(skip_serializing_if = "Option::is_none")]
    pub relay_message: Option<RelayMessage>

}


impl ClientMessage {
    pub fn new() -> ClientMessage {
        ClientMessage{

            register: None,

            abort: None,

            relay_message: None

        }
    }

    pub fn register(&mut self, protocol_id: ProtocolIdentifier, capacity: u32) {
        self.register = Some(RegisterMessage{
            protocol_id,
            capacity,
        });

    }

    pub fn is_empty(&self) -> bool{
        self.relay_message.is_none() && self.abort.is_none() && self.register.is_none()
    }

    pub fn are_equal_payloads(&self, msg: &ClientMessage) -> bool {
        if self.register.is_some() && msg.register.is_some(){return true;}
        else if self.relay_message.is_some() && msg.relay_message.is_some(){
            let self_message = self.relay_message.clone().unwrap().message;
            let message = msg.relay_message.clone().unwrap().message;
            return self_message == message;
        }
        else if self.abort.is_some() && msg.abort.is_some() {
            return true;
        }
        false
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct RegisterResponse {
    peer_number: PeerIdentifier
}

// in: clientMessage out:serverMessage
pub type ServerToClientCodec= codec::LengthPrefixedJson<ClientMessage, ServerMessage>;
pub type ClientToServerCodec = codec::LengthPrefixedJson<ServerMessage, ClientMessage>;


// codec for register message
//pub type ServerToClientRegister = codec::LengthPrefixedJson<RegisterMessage, RegisterResponse>;
//pub type ClientToServerRegister = codec::LengthPrefixedJson<RegisterResponse, RegisterMessage>;

