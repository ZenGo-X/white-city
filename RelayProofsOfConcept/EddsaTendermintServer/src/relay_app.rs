use crate::relay_session::RelaySession;
use abci::{RequestCheckTx, RequestQuery, ResponseCheckTx, ResponseQuery};
use log::{debug, error, info, warn};
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

impl abci::Application for RelayApp {
    fn check_tx(&mut self, req: &RequestCheckTx) -> ResponseCheckTx {
        let mut resp = ResponseCheckTx::new();
        let c = convert_tx(req.get_tx());
        println!("Received {:?}", c);
        let client_message: ClientMessage = serde_json::from_slice(req.get_tx()).unwrap();
        println!("Value is {:?}", client_message);

        match client_message.msg_type() {
            ClientMessageType::Register => {
                let register = client_message.register.unwrap();
                warn!(
                    "Got register message. protocol id requested: {}",
                    register.protocol_id
                );
                let messages_to_send = self.relay_session.register_new_peer(
                    // This is a palce holder, client can sent its address/pub key/nonce
                    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
                    register.protocol_id,
                    register.capacity,
                );
            }
            _ => unimplemented!("This is not yet implemented"),
        }

        resp.set_code(0);
        resp
    }

    fn query(&mut self, req: &RequestQuery) -> ResponseQuery {
        let mut resp = ResponseQuery::new();
        resp.set_code(0);
        println!("Code is {}", resp.get_code());
        resp.set_log(String::from("Exists"));
        resp.set_index(-1);
        resp.set_height(1_i64);
        resp.set_codespace(String::from("Bla"));
        resp
    }
}
