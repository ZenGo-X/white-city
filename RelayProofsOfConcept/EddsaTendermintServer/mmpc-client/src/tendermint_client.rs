use std::collections::BTreeMap;
use std::net::SocketAddr;

use crate::eddsa_peer::ProtocolDataManager;
use crate::peer::{Peer, MAX_CLIENTS};
use log::{debug, error, info, warn};

use mmpc_server_common::common::*;
use mmpc_server_common::{
    ClientMessage, MessagePayload, MissingMessagesRequest, PeerIdentifier, ProtocolIdentifier,
    RelayMessage, ServerMessage, ServerMessageType, ServerResponse, StoredMessages,
};

pub struct SessionClient<T>
where
    T: Peer,
{
    pub state: State<T>,
    pub client: tendermint::rpc::Client,
}

impl<T: Peer> SessionClient<T> {
    pub fn new(
        client_addr: SocketAddr,
        server_addr: &tendermint::net::Address,
        capacity: u32,
    ) -> SessionClient<T> {
        let protocol_id = 1;
        SessionClient {
            state: State::new(protocol_id, capacity, client_addr),
            client: tendermint::rpc::Client::new(server_addr).unwrap(),
        }
    }
}

impl<T: Peer> SessionClient<T> {
    pub fn query(&self) -> BTreeMap<u32, ClientMessage> {
        let current_step = self.state.data_manager.data_holder.current_step();
        println!("Current step {}", current_step);
        let capacity = self.state.data_manager.capacity;
        println!("Capacity {}", capacity);
        let mut missing_clients = self
            .state
            .stored_messages
            .get_missing_clients_vector(current_step, capacity);

        println!("Missing: {:?}", missing_clients);

        // No need to query if nothing is missing
        if missing_clients.is_empty() {
            return BTreeMap::new();
        }

        if missing_clients.len() > MAX_CLIENTS {
            missing_clients.truncate(MAX_CLIENTS);
        }
        println!("Missing requested: {:?}", missing_clients);

        let request = MissingMessagesRequest {
            round: current_step,
            missing_clients: missing_clients,
        };
        let tx = serde_json::to_string(&request).unwrap();
        let response = self.client.abci_query(None, tx, None, false).unwrap();
        println!("RawResponse: {:?}", response);
        let server_response = response.log;
        let server_response: BTreeMap<u32, ClientMessage> =
            match serde_json::from_str(&server_response.to_string()) {
                Ok(server_response) => server_response,
                Err(_) => BTreeMap::new(),
            };
        return server_response;
    }

    pub fn register(&mut self, index: u32, capacity: u32) -> ServerMessage {
        let mut msg = ClientMessage::new();
        let port = 8080 + index;
        let client_addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
        // No index to begin with
        msg.register(client_addr, self.state.protocol_id, capacity, -1);

        debug!("Regsiter message {:?}", msg);
        let tx =
            tendermint::abci::transaction::Transaction::new(serde_json::to_string(&msg).unwrap());
        let response = self.client.broadcast_tx_commit(tx).unwrap();
        let server_response = response.clone().deliver_tx.log.unwrap();
        info!("Registered OK");
        debug!("ServerResponse {:?}", server_response);
        let server_response: ServerMessage =
            serde_json::from_str(&response.deliver_tx.log.unwrap().to_string()).unwrap();
        debug!("ServerResponse {:?}", server_response);
        // TODO Add Error checks etc
        self.state.registered = true;
        return server_response;
    }

    pub fn send_message(&self, msg: ClientMessage) -> BTreeMap<u32, ClientMessage> {
        debug!("Sending message {:?}", msg);
        let tx =
            tendermint::abci::transaction::Transaction::new(serde_json::to_string(&msg).unwrap());
        let response = self.client.broadcast_tx_commit(tx).unwrap();
        let server_response = response.clone().deliver_tx.log.unwrap();
        debug!("ServerResponse {:?}", server_response);
        let server_response: BTreeMap<u32, ClientMessage> =
            serde_json::from_str(&response.deliver_tx.log.unwrap().to_string()).unwrap();
        return server_response;
    }

    // Stores the server response to the stored messages
    pub fn store_server_response(&mut self, messages: &BTreeMap<u32, ClientMessage>) {
        let round = self.state.data_manager.data_holder.current_step();
        for (client_idx, msg) in messages {
            self.state
                .stored_messages
                .update(round, *client_idx, msg.clone());
        }
    }

    pub fn handle_relay_message(&mut self, client_msg: ClientMessage) {
        let msg = client_msg.relay_message.unwrap();
        self.state.handle_relay_message(msg.clone());
    }

    pub fn generate_client_answer(&mut self, msg: ServerMessage) -> Option<ClientMessage> {
        // let last_message = self.state.last_message.clone();
        let mut new_message = None;
        let msg_type = msg.msg_type();
        match msg_type {
            ServerMessageType::Response => {
                let next = self.state.handle_server_response(&msg);
                match next {
                    Ok(next_msg) => {
                        new_message = Some(next_msg.clone());
                    }
                    Err(_) => {
                        error!("Error in handle_server_response");
                    }
                }
            }
            // TODO: better cases separation, this is a placeholder
            ServerMessageType::RelayMessage => {
                new_message = Some(ClientMessage::new());
            }
            //     let next = self.state.handle_relay_message(msg.clone());
            //     match next {
            //         Some(next_msg) => {
            //             //debug!("next message to send is {:}", next_msg);
            //             new_message = Some(self.state.generate_relay_message(next_msg.clone()));
            //         }
            //         None => {
            //             debug!("next item is None. Client is finished.");
            //             new_message = Some(ClientMessage::new());
            //         }
            //     }
            // }
            ServerMessageType::Abort => {
                info!("Got abort message");
                //Ok(MessageProcessResult::NoMessage)
                new_message = Some(ClientMessage::new());
            }
            ServerMessageType::Undefined => {
                new_message = Some(ClientMessage::new());
                //panic!("Got undefined message: {:?}",msg);
            }
        };
        new_message
    }
}

/// Inner client state, responsible for parsing server responses and producing the next message
pub struct State<T>
where
    T: Peer,
{
    pub registered: bool,
    pub protocol_id: ProtocolIdentifier,
    pub client_addr: SocketAddr,
    pub data_manager: ProtocolDataManager<T>,
    pub last_message: ClientMessage,
    pub bc_dests: Vec<ProtocolIdentifier>,
    pub stored_messages: StoredMessages,
}

impl<T: Peer> State<T> {
    pub fn new(protocol_id: ProtocolIdentifier, capacity: u32, client_addr: SocketAddr) -> State<T>
    where
        T: Peer,
    {
        let data_m: ProtocolDataManager<T> = ProtocolDataManager::new(capacity);
        State {
            registered: false,
            protocol_id,
            client_addr,
            last_message: ClientMessage::new(),
            bc_dests: (1..(capacity + 1)).collect(),
            data_manager: data_m,
            stored_messages: StoredMessages::new(),
        }
    }
}

impl<T: Peer> State<T> {
    fn handle_relay_message(&mut self, relay_msg: RelayMessage) -> Option<MessagePayload> {
        // parse relay message
        let from = relay_msg.peer_number;
        if from == self.data_manager.peer_id {
            debug!("-------self message accepted ------\n ");
        }
        let payload = relay_msg.message;
        self.data_manager.get_next_message(from, payload)
    }

    fn generate_relay_message(&self, payload: MessagePayload) -> ClientMessage {
        let _msg = ClientMessage::new();
        // create relay message
        let mut relay_message = RelayMessage::new(
            self.data_manager.peer_id,
            self.protocol_id,
            self.client_addr,
        );
        let to: Vec<u32> = self.bc_dests.clone();

        let mut client_message = ClientMessage::new();

        relay_message.set_message_params(to, String::from(payload));
        client_message.relay_message = Some(relay_message);
        client_message
    }

    fn handle_register_response(&mut self, peer_id: PeerIdentifier) -> Result<ClientMessage, ()> {
        info!("Peer identifier: {}", peer_id);
        // Set the session parameters
        let message = self
            .data_manager
            .initialize_data(peer_id)
            .unwrap_or_else(|| panic!("failed to initialize"));
        Ok(self.generate_relay_message(message.clone()))
    }

    fn get_last_message(&self) -> Option<ClientMessage> {
        let last_msg = self.last_message.clone();
        return Some(last_msg.clone());
    }

    fn handle_error_response(&mut self, err_msg: &str) -> Result<ClientMessage, &'static str> {
        match err_msg {
            resp if resp == String::from(NOT_YOUR_TURN) => {
                let last_msg = self.get_last_message();
                match last_msg {
                    Some(msg) => {
                        return Ok(msg.clone());
                    }
                    None => {
                        panic!("No message to resend");
                    }
                }
            }
            not_initialized_resp if not_initialized_resp == String::from(STATE_NOT_INITIALIZED) => {
                debug!("Not initialized, sending again");
                let last_msg = self.get_last_message();
                match last_msg {
                    Some(_) => {
                        // If protocol is not initialized, wait for a message from the server
                        return Ok(ClientMessage::new());
                    }
                    None => {
                        panic!("No message to resend");
                    }
                }
            }
            _ => {
                warn!("didn't handle error correctly");
                return Err("error response handling failed");
            }
        }
    }

    fn handle_server_response(
        &mut self,
        msg: &ServerMessage,
    ) -> Result<ClientMessage, &'static str> {
        let server_response = msg.response.clone().unwrap();
        match server_response {
            ServerResponse::Register(peer_id) => {
                let client_message = self.handle_register_response(peer_id);
                match client_message {
                    Ok(_msg) => {
                        debug!("sending peers first message: {:#?}", _msg);
                        return Ok(_msg.clone());
                    }
                    Err(_) => {
                        error!("error occured");
                        return Ok(ClientMessage::new());
                    }
                }
            }
            ServerResponse::ErrorResponse(err_msg) => {
                let err_msg_slice: &str = &err_msg[..];
                let msg = self.handle_error_response(err_msg_slice);
                match msg {
                    Ok(_msg) => return Ok(_msg),
                    Err(_) => {
                        error!("error occured");
                        return Ok(ClientMessage::new());
                    }
                }
            }
            ServerResponse::NoResponse => unimplemented!(),
        }
    }
}
