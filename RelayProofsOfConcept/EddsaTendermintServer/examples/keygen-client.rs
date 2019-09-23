use log::{debug, error, info, warn};
use std::net::SocketAddr;
use std::{thread, time};

use relay_server_common::{
    ClientMessage, MessagePayload, PeerIdentifier, ProtocolIdentifier, RelayMessage, ServerMessage,
    ServerMessageType, ServerResponse,
};

use curv::elliptic::curves::ed25519::*;
use curv::GE;
use multi_party_ed25519::protocols::aggsig::{EphemeralKey, KeyAgg, KeyPair};
use relay_server_common::common::*;
use std::collections::HashMap;
use std::fs;

use clap::{App, Arg, ArgMatches};

#[allow(non_snake_case)]
struct EddsaPeer {
    // this peers identifier in this session
    pub peer_id: PeerIdentifier,
    // # of participants
    pub capacity: u32,

    pub current_step: u32,
    // is peer done with all calculations
    pub is_done: bool,

    // eddsa data
    pub client_key: KeyPair,
    pub pks: HashMap<PeerIdentifier, Ed25519Point>,
    pub commitments: HashMap<PeerIdentifier, String>,
    pub r_s: HashMap<PeerIdentifier, String>,
    pub sigs: HashMap<PeerIdentifier, String>,
    pub ephemeral_key: Option<EphemeralKey>,

    pub agg_key: Option<KeyAgg>,
    pub R_tot: Option<GE>,

    // indicators for which of this peers messages were accepted
    pub pk_accepted: bool,
    pub commitment_accepted: bool,
    pub r_accepted: bool,
    pub sig_accepted: bool,

    // messages this peer generates
    pub pk_msg: Option<MessagePayload>,
    pub commitment_msg: Option<MessagePayload>,
    pub r_msg: Option<MessagePayload>,
    pub sig_msg: Option<MessagePayload>,
}

impl EddsaPeer {
    /// inner calculations & data manipulations
    fn add_pk(&mut self, peer_id: PeerIdentifier, pk: Ed25519Point) {
        self.pks.insert(peer_id, pk);
    }
    fn aggregate_pks(&mut self) -> KeyAgg {
        debug!("aggregating pks");
        let _cap = self.capacity as usize;
        let mut pks = Vec::with_capacity(self.capacity as usize);
        for index in 0..self.capacity {
            let peer = index + 1;
            let pk = self.pks.get_mut(&peer).unwrap();
            pks.push(pk.clone());
        }
        debug!("# of public keys : {:?}", pks.len());
        let peer_id = self.peer_id;
        let index = (peer_id - 1) as usize;
        let agg_key = KeyPair::key_aggregation_n(&pks, &index);
        return agg_key;
    }
}

impl EddsaPeer {
    /// data updaters for each step
    pub fn update_data_step_0(&mut self, from: PeerIdentifier, payload: MessagePayload) {
        let payload_type = EddsaPeer::resolve_payload_type(&payload);
        match payload_type {
            MessagePayloadType::PublicKey(pk) => {
                let peer_id = self.peer_id;
                if from == peer_id {
                    self.pk_accepted = true;
                }
                let s_slice: &str = &pk[..]; // take a full slice of the string
                let _pk = serde_json::from_str(s_slice);
                info!("-------Got peer # {:} pk! {:?}", from, pk);
                match _pk {
                    Ok(_pk) => self.add_pk(from, _pk),
                    Err(_) => panic!("Could not serialize public key"),
                }
            }
        }
    }
}

impl EddsaPeer {
    fn is_step_done(&mut self) -> bool {
        match self.current_step {
            0 => return self.is_done_step_0(),
            _ => panic!("Unsupported step"),
        }
    }
    pub fn is_done_step_0(&mut self) -> bool {
        if self.pks.len() == self.capacity as usize {
            self.finalize().expect("Finalized falied");
            return true;
        }
        false
    }
}

impl EddsaPeer {
    pub fn resolve_payload_type(message: &MessagePayload) -> MessagePayloadType {
        let msg_payload = message.clone();

        let split_msg: Vec<&str> = msg_payload.split(RELAY_MESSAGE_DELIMITER).collect();
        let msg_prefix = split_msg[0];
        let msg_payload = String::from(split_msg[1].clone());
        match msg_prefix {
            pk_prefix if pk_prefix == String::from(PK_MESSAGE_PREFIX) => {
                return MessagePayloadType::PublicKey(msg_payload);
            }
            _ => panic!("Unknown relay message prefix"),
        }
    }
}

impl Peer for EddsaPeer {
    fn new(capacity: u32) -> EddsaPeer {
        EddsaPeer {
            client_key: KeyPair::create(),
            pks: HashMap::new(),
            commitments: HashMap::new(),
            r_s: HashMap::new(),
            sigs: HashMap::new(),
            capacity,
            peer_id: 0,
            agg_key: None,
            current_step: 0,
            R_tot: None,
            ephemeral_key: None,
            pk_accepted: false,
            commitment_accepted: false,
            r_accepted: false,
            sig_accepted: false,
            is_done: false,

            pk_msg: None,
            commitment_msg: None,
            r_msg: None,
            sig_msg: None,
        }
    }

    fn zero_step(&mut self, peer_id: PeerIdentifier) -> Option<MessagePayload> {
        self.peer_id = peer_id;
        let pk = self.client_key.public_key.clone();

        let pk_s = serde_json::to_string(&pk).expect("Failed in serialization");

        self.pk_msg = Some(generate_pk_message_payload(&pk_s));
        return self.pk_msg.clone();
    }

    fn do_step(&mut self) {
        debug!("Current step is: {:}", self.current_step);
        if self.is_step_done() {
            // do the next step
            debug!("step {:} done!", self.current_step);
            self.current_step += 1;
            match self.current_step {
                1 => {
                    info!("----------\nDone.\n----------");
                    self.is_done = true;
                }
                _ => panic!("Unsupported step"),
            }
        } else {
            debug!("step not done");
        }
    }

    fn update_data(&mut self, from: PeerIdentifier, payload: MessagePayload) {
        // update data according to step
        match self.current_step {
            0 => self.update_data_step_0(from, payload),

            _ => panic!("Unsupported step"),
        }
    }
    /// Does the final calculation of the protocol
    /// in this case:
    ///     collection all signatures
    ///     and verifying the message
    fn finalize(&mut self) -> Result<(), &'static str> {
        let key = &self.client_key.clone();
        let apk = &self.aggregate_pks();
        let index = &self.peer_id;

        let keygen_json = serde_json::to_string(&(key, apk, index)).unwrap();

        let res = fs::write(format!("keys{}", self.peer_id), keygen_json);
        match res {
            Ok(_) => Ok(()),
            Err(_) => Err("Failed to verify"),
        }
    }
    /// check that the protocol is done
    /// and that this peer can finalize its calculations
    fn is_done(&mut self) -> bool {
        self.is_done_step_0()
    }

    /// get the next item the peer needs to send
    /// depending on the current step and the last message
    /// of the peer that was accepted by the server
    fn get_next_item(&mut self) -> Option<MessagePayload> {
        //println!("current_step: {:}, pk_accepted: {:} commitment_accepted: {:} r_accepted: {:} sig_accepted: {:}",self.current_step,self.pk_accepted,self.commitment_accepted, self.r_accepted, self.sig_accepted);
        if self.current_step == 0 || !self.pk_accepted {
            debug!("next item is pk: {:?}", self.pk_msg);
            return self.pk_msg.clone();
        }
        None
    }
}
pub trait Peer {
    fn new(capacity: u32) -> Self;
    fn zero_step(&mut self, peer_id: PeerIdentifier) -> Option<MessagePayload>;
    fn do_step(&mut self);
    fn update_data(&mut self, from: PeerIdentifier, payload: MessagePayload);
    fn get_next_item(&mut self) -> Option<MessagePayload>;
    fn finalize(&mut self) -> Result<(), &'static str>;
    fn is_done(&mut self) -> bool;
}

struct ProtocolDataManager<T: Peer> {
    pub peer_id: PeerIdentifier,
    pub capacity: u32,
    pub current_step: u32,
    pub data_holder: T, // will be filled when initializing, and on each new step
    pub client_data: Option<MessagePayload>, // new data calculated by this peer at the beginning of a step (that needs to be sent to other peers)
    pub new_client_data: bool,
}

impl<T: Peer> ProtocolDataManager<T> {
    pub fn new(capacity: u32) -> ProtocolDataManager<T>
    where
        T: Peer,
    {
        ProtocolDataManager {
            peer_id: 0,
            current_step: 0,
            capacity,
            data_holder: Peer::new(capacity),
            client_data: None,
            new_client_data: false,
        }
    }

    /// set manager with the initial values that a local peer holds at the beginning of
    /// the protocol session
    /// return: first message
    pub fn initialize_data(&mut self, peer_id: PeerIdentifier) -> Option<MessagePayload> {
        self.peer_id = peer_id;
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

fn arg_matches<'a>() -> ArgMatches<'a> {
    App::new("relay-server")
        .arg(
            Arg::with_name("index")
                .short("I")
                .long("index")
                .default_value("1"),
        )
        .arg(
            Arg::with_name("capacity")
                .default_value("2")
                .short("P")
                .long("capacity"),
        )
        .arg(
            Arg::with_name("filename")
                .default_value("keys")
                .long("filename")
                .short("F"),
        )
        .arg(
            Arg::with_name("proxy")
                .default_value("127.0.0.1:26657")
                .long("proxy"),
        )
        .get_matches()
}

struct SessionClient {
    pub state: State<EddsaPeer>,
    pub client: tendermint::rpc::Client,
}

impl SessionClient {
    pub fn new(
        client_addr: SocketAddr,
        server_addr: &tendermint::net::Address,
        capacity: u32,
    ) -> SessionClient {
        let protocol_id = 1;
        SessionClient {
            state: State::new(protocol_id, capacity, client_addr),
            client: tendermint::rpc::Client::new(server_addr).unwrap(),
        }
    }
}

/// Inner client state, responsible for parsing server responses and producing the next message
struct State<T>
where
    T: Peer,
{
    pub registered: bool,
    pub protocol_id: ProtocolIdentifier,
    pub client_addr: SocketAddr,
    pub data_manager: ProtocolDataManager<T>,
    pub last_message: ClientMessage,
    pub bc_dests: Vec<ProtocolIdentifier>,
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

pub enum MessageProcessResult {
    Message,
    NoMessage,
    Abort,
}

impl SessionClient {
    pub fn query(&self) -> Vec<ClientMessage> {
        let tx = "0";
        let response = self.client.abci_query(None, tx, None, false).unwrap();
        debug!("RawResponse: {:?}", response);
        let server_response = response.log;
        let empty_vec = Vec::new();
        let server_response: Vec<ClientMessage> =
            match serde_json::from_str(&server_response.to_string()) {
                Ok(server_response) => server_response,
                Err(_) => empty_vec,
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

    pub fn send_message(&self, msg: ClientMessage) -> Vec<ClientMessage> {
        debug!("Sending message {:?}", msg);
        let tx =
            tendermint::abci::transaction::Transaction::new(serde_json::to_string(&msg).unwrap());
        let response = self.client.broadcast_tx_commit(tx).unwrap();
        let server_response = response.clone().deliver_tx.log.unwrap();
        debug!("ServerResponse {:?}", server_response);
        let server_response: Vec<ClientMessage> =
            serde_json::from_str(&response.deliver_tx.log.unwrap().to_string()).unwrap();
        return server_response;
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

#[derive(Debug)]
enum MessagePayloadType {
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

fn main() {
    better_panic::Settings::debug()
        .most_recent_first(false)
        .lineno_suffix(true)
        .install();

    let matches = arg_matches();

    let index: u32 = matches
        .value_of("index")
        .unwrap()
        .parse()
        .expect("Unable to parse index");

    let capacity: u32 = matches
        .value_of("capacity")
        .unwrap()
        .parse()
        .expect("Invalid number of participants");

    let proxy: String = matches
        .value_of("proxy")
        .unwrap()
        .parse()
        .expect("Invalid proxy address");

    let port = 8080 + index;
    let proxy_addr = format!("tcp://{}", proxy);
    let client_addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let mut session = SessionClient::new(client_addr, &proxy_addr.parse().unwrap(), capacity);
    let server_response = session.register(index, capacity);
    let next_message = session.generate_client_answer(server_response);
    debug!("Next message: {:?}", next_message);
    // TODO The client/server response could be an error
    let server_response = session.send_message(next_message.unwrap());
    debug!("Server Response: {:?}", server_response);
    //session.query();
    if server_response.len() == capacity as usize {
        for msg in server_response {
            session.handle_relay_message(msg.clone());
        }
    } else {
        loop {
            let server_response = session.query();
            thread::sleep(time::Duration::from_millis(100));
            if server_response.len() == capacity as usize {
                for msg in server_response {
                    session.handle_relay_message(msg.clone());
                }
                return;
            }
        }
    }
}
