use crate::relay_session::RelaySession;
use abci::{
    RequestCheckTx, RequestDeliverTx, RequestQuery, ResponseCheckTx, ResponseDeliverTx,
    ResponseQuery,
};
use log::{debug, info, warn};
use relay_server_common::protocol::ProtocolDescriptor;
use relay_server_common::{ClientMessage, ClientMessageType, ServerMessage, ServerResponse};

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
    fn can_relay(&self, client_message: &ClientMessage) -> u32 {
        match client_message.msg_type() {
            ClientMessageType::RelayMessage => {
                let msg = client_message.clone().relay_message.unwrap();
                let can_relay = self.relay_session.can_relay(&msg.from, &msg);
                match can_relay {
                    Ok(()) => debug!("Can relay this message"),
                    _ => (),
                }
            }
            _ => (),
        }
        0
    }

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
            ClientMessageType::RelayMessage => {
                // TODO: Check validity of relay message here
                0
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

        debug!("Message type is {:?}", client_message.msg_type());

        match client_message.msg_type() {
            ClientMessageType::Register => {
                if self.is_valid(&client_message) != 0 {
                    resp.set_code(1);
                    return resp;
                }
                let register = client_message.register.unwrap();
                warn!(
                    "Got register message. protocol id requested: {}",
                    register.protocol_id
                );
                let client_index = self
                    .relay_session
                    .register_new_peer(
                        register.addr,
                        register.protocol_id,
                        register.capacity,
                        register.index,
                    )
                    .unwrap();
                resp.set_code(0);
                info!("Setting data to {:?}", resp.data);
                let mut server_msg = ServerMessage::new();
                server_msg.response = Some(ServerResponse::Register(client_index));
                // TODO: Currently using log and not data, data is expecting a different encoding,
                // sigh
                resp.set_log(serde_json::to_string(&server_msg).unwrap().to_owned());
            }
            ClientMessageType::RelayMessage => {
                let relay_msg = client_message.clone().relay_message.unwrap();
                let peer_id = relay_msg.peer_number;
                info!("Got relay message from {}", peer_id);
                if self.can_relay(&client_message) == 0 {
                    debug!("I can relay this")
                }
                let round = self.relay_session.round();
                self.relay_session
                    .update_stored_messages(round, peer_id, client_message);
                let stored_messages = self.relay_session.stored_messages();
                let mut response_vec = Vec::new();

                if stored_messages.get_number_messages(&round)
                    == self.relay_session.protocol().capacity as usize
                {
                    response_vec = stored_messages.get_messages_vector_client_message(&round);
                    resp.set_log(serde_json::to_string(&response_vec).unwrap().to_owned());
                    debug!("Response log {:?}", resp.log);
                    // If received a message from each party, increase round
                    self.relay_session.increase_step();
                } else {
                    resp.set_log(
                        serde_json::to_string(&response_vec.clear())
                            .unwrap()
                            .to_owned(),
                    );
                }
                resp.set_log(serde_json::to_string(&response_vec).unwrap().to_owned());
                debug!("Response log {:?}", resp.log);
            }
            _ => unimplemented!("This is not yet implemented"),
        }

        resp
    }

    fn query(&mut self, req: &RequestQuery) -> ResponseQuery {
        let mut resp = ResponseQuery::new();

        let c = convert_tx(&req.data);
        info!("Received {:?} In Query", c);

        // TODO: Error handle
        let requested_round = c.parse::<u32>().unwrap();

        let stored_messages = self.relay_session.stored_messages();
        let mut response_vec = Vec::new();

        debug!("All messages {:?}", stored_messages);

        if stored_messages.get_number_messages(&requested_round)
            == self.relay_session.protocol().capacity as usize
        {
            response_vec = stored_messages.get_messages_vector_client_message(&requested_round);
            // resp.set_log("Some string".to_owned());
            resp.set_log(serde_json::to_string(&response_vec).unwrap().to_owned());
            debug!("Response log {:?}", resp.log);
        } else {
            resp.set_log(
                serde_json::to_string(&response_vec.clear())
                    .unwrap()
                    .to_owned(),
            );
        }

        resp.set_code(0);
        resp.set_index(-1);
        resp.set_height(1_i64);
        resp
    }
}
