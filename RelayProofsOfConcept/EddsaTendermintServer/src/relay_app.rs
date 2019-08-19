use crate::relay_session::RelaySession;
use abci::{
    RequestCheckTx, RequestDeliverTx, RequestQuery, ResponseCheckTx, ResponseDeliverTx,
    ResponseQuery,
};
use log::{debug, error, info, warn};
use relay_server_common::protocol::ProtocolDescriptor;
use relay_server_common::{ClientMessage, ClientMessageType, ServerMessage};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub struct RelayApp {
    relay_session: RelaySession,
}

impl RelayApp {
    pub fn new(capacity: u32) -> RelayApp {
        RelayApp {
            relay_session: RelaySession::new(capacity),
        }
    }
}

// Convert incoming tx data to the proper BigEndian size. txs.len() > 8 will return 0
fn convert_tx(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("Found invalid UTF-8")
}

impl RelayApp {
    fn is_valid(&self, client_message: &ClientMessage) -> u32 {
        match client_message.msg_type() {
            ClientMessageType::Register => {
                let register = client_message.clone().register.unwrap();
                info!(
                    "Got register message. protocol id requested: {}",
                    register.protocol_id
                );
                let protocol_descriptor =
                    ProtocolDescriptor::new(register.protocol_id, register.capacity);
                if self
                    .relay_session
                    .can_register(&register.addr, protocol_descriptor)
                {
                    0
                } else {
                    1
                }
            }
            _ => unimplemented!("This is not yet implemented"),
        }
    }
}

impl abci::Application for RelayApp {
    fn check_tx(&mut self, req: &RequestCheckTx) -> ResponseCheckTx {
        let mut resp = ResponseCheckTx::new();
        let c = convert_tx(req.get_tx());
        info!("Received {:?}", c);
        let client_message: ClientMessage = serde_json::from_slice(req.get_tx()).unwrap();
        info!("Value is {:?}", client_message);
        resp.set_code(self.is_valid(&client_message));
        resp
    }

    fn deliver_tx(&mut self, req: &RequestDeliverTx) -> ResponseDeliverTx {
        let mut resp = ResponseDeliverTx::new();
        let c = convert_tx(req.get_tx());
        info!("Received {:?} In DeliverTx", c);
        let client_message: ClientMessage = serde_json::from_slice(req.get_tx()).unwrap();
        info!("Value is {:?} In DeliverTx", client_message);

        if self.is_valid(&client_message) != 0 {
            resp.set_code(1);
            return resp;
        }

        match client_message.msg_type() {
            ClientMessageType::Register => {
                let register = client_message.register.unwrap();
                warn!(
                    "Got register message. protocol id requested: {}",
                    register.protocol_id
                );
                let client_index = self
                    .relay_session
                    .register_new_peer(register.addr, register.protocol_id, register.capacity)
                    .unwrap();
                resp.set_code(0);
                info!("Setting data to {:?}", resp.data);
                resp.set_log(client_index.to_string());
                // TODO enable for higher numbers, set data and not log
                // resp.set_data(vec![client_index as u8]);
            }
            _ => unimplemented!("This is not yet implemented"),
        }

        resp
    }

    fn query(&mut self, req: &RequestQuery) -> ResponseQuery {
        let mut resp = ResponseQuery::new();

        let c = convert_tx(&req.data);
        info!("Received {:?} In Query", c);
        let client_message: ClientMessage = serde_json::from_slice(&req.data).unwrap();
        info!("Value is {:?} In Query", client_message);

        resp.set_code(0);
        info!("Code is {}", resp.get_code());
        resp.set_log(String::from("Exists"));
        resp.set_index(-1);
        resp.set_height(1_i64);
        resp.set_codespace(String::from("Bla"));
        resp
    }
}
