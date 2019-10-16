use crate::relay_session::RelaySession;
use abci::{
    RequestCheckTx, RequestDeliverTx, RequestQuery, ResponseCheckTx, ResponseDeliverTx,
    ResponseQuery,
};
use log::{debug, info, warn};
use mmpc_server_common::protocol::ProtocolDescriptor;
use mmpc_server_common::{
    ClientMessage, ClientMessageType, MissingMessagesRequest, ServerMessage, ServerResponse,
};

const MAX_CLIENTS: usize = 12;

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
                info!("Stored message of client {}", peer_id);

                let response = self
                    .relay_session
                    .stored_messages()
                    .get_messages_map_client_message(round);
                resp.set_log(serde_json::to_string(&response).unwrap().to_owned());
                debug!("Response log {:?}", resp.log);
                self.relay_session
                    .try_increase_round(self.relay_session.protocol().capacity);
                // If received a message from each party, increase round
                debug!("Response log {:?}", resp.log);
            }
            _ => unimplemented!("This is not yet implemented"),
        }

        resp
    }

    fn query(&mut self, req: &RequestQuery) -> ResponseQuery {
        let mut resp = ResponseQuery::new();

        let missing_messages: MissingMessagesRequest = serde_json::from_slice(&req.data).unwrap();
        info!("Received {:?} In Query", missing_messages);

        // TODO: Error handle
        let requested_round = missing_messages.round;
        let mut missing_clients = missing_messages.missing_clients;
        info!("Requested round {}", requested_round);

        let stored_messages = self.relay_session.stored_messages();

        if missing_clients.len() > MAX_CLIENTS {
            missing_clients.truncate(MAX_CLIENTS);
        }
        let response =
            stored_messages.get_messages_map_from_vector(requested_round, &missing_clients);

        match stored_messages.messages.get(&1) {
            Some(test) => info!("Stored messages in round 1: {:?}", test),
            None => {}
        }

        info!("Server response {:?}", response);

        resp.set_log(serde_json::to_string(&response).unwrap().to_owned());
        info!("Response log {:?}", resp.log);

        resp.set_code(0);
        resp.set_index(-1);
        resp.set_height(1_i64);
        resp
    }
}
