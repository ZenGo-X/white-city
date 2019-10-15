/// Implementation of a client that communicates with the relay server
/// This client represents eddsa peer
///
///
use std::cell::RefCell;
use std::net::SocketAddr;
use std::vec::Vec;
use std::{thread, time};

use relay_server_common::{
    ClientMessage, MessagePayload, MissingMessagesRequest, PeerIdentifier, ProtocolIdentifier,
    RelayMessage, ServerMessage, ServerMessageType, ServerResponse, StoredMessages,
};

use curv::arithmetic::traits::Converter;
use curv::elliptic::curves::ed25519::*;
use curv::elliptic::curves::traits::ECPoint;
use curv::elliptic::curves::traits::ECScalar;
use curv::{BigInt, FE, GE};
use multi_party_eddsa::protocols::aggsig::{
    test_com, verify, EphemeralKey, KeyAgg, KeyPair, SignFirstMsg, SignSecondMsg, Signature,
};

use relay_server_common::common::*;

use std::collections::{BTreeMap, HashMap};
use std::fs;

use clap::{App, Arg, ArgMatches};

const MAX_CLIENTS: usize = 12;

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
    // message to sign
    pub message: Vec<u8>,

    pub agg_key: Option<KeyAgg>,
    pub kg_index: u32,
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
    fn add_commitment(&mut self, peer_id: PeerIdentifier, commitment: String) {
        self.commitments.insert(peer_id, commitment);
    }
    fn add_r(&mut self, peer_id: PeerIdentifier, r: String) {
        //let v = (r,blind_factor);
        self.r_s.insert(peer_id, r);
    }
    fn add_sig(&mut self, peer_id: PeerIdentifier, sig: String) {
        self.sigs.insert(peer_id, sig);
    }
    fn compute_r_tot(&mut self) -> GE {
        #[allow(non_snake_case)]
        let mut Ri: Vec<GE> = Vec::new();
        for (_peer_id, r) in &self.r_s {
            let r_slice: &str = &r[..];
            let r: SignSecondMsg =
                serde_json::from_str(r_slice).unwrap_or_else(|_| panic!("Serialization error"));
            Ri.push(r.R.clone());
        }
        let r_tot = Signature::get_R_tot(Ri);
        return r_tot;
    }
    fn aggregate_pks(&mut self) -> KeyAgg {
        println!("aggregating pks");
        let _cap = self.capacity as usize;
        let mut pks = Vec::with_capacity(self.capacity as usize);
        for index in 0..self.capacity {
            let peer = index + 1;
            let pk = self.pks.get_mut(&peer).unwrap();
            pks.push(pk.clone());
        }
        println!("# of public keys : {:?}", pks.len());
        let peer_id = self.peer_id;
        let index = (peer_id - 1) as usize;
        println!("Public keys {:?}", &pks);
        println!("KG index:{}, SIG index:{}", self.kg_index, peer_id);
        // TODO: sort the pks according to key-gen indexes when applying
        KeyPair::key_aggregation_n(&pks, &index)
    }

    fn validate_commitments(&mut self) -> bool {
        // iterate over all peer Rs
        println!("----------\nvalidating commitments\n----------");
        let eight: FE = ECScalar::from(&BigInt::from(8));
        let eight_inv = eight.invert();
        let r_s = &self.r_s;
        for (peer_id, r) in r_s {
            println!("peer: {:}", peer_id);
            println!("r: {:}", r);
            // convert the json_string to a construct
            let _r: SignSecondMsg = serde_json::from_str(r).unwrap();

            // get the corresponding commitment
            let k = peer_id.clone();
            let cmtmnt = self
                .commitments
                .get(&k)
                .expect("peer didn't send commitment");
            println!("commitment : {:?}", cmtmnt);
            let commitment: SignFirstMsg = serde_json::from_str(cmtmnt).unwrap();
            // if we couldn't validate the commitment - failure
            if !test_com(
                &(_r.R * eight_inv),
                &_r.blind_factor,
                &commitment.commitment,
            ) {
                return false;
            }
        }
        println!("----------\ncommitments valid\n----------");
        true
    }
}

impl EddsaPeer {
    /// data updaters for each step
    pub fn update_data_step_0(&mut self, from: PeerIdentifier, payload: MessagePayload) {
        let payload_type = EddsaPeer::resolve_payload_type(&payload);
        let eight: FE = ECScalar::from(&BigInt::from(8));
        let eight_inv = eight.invert();
        match payload_type {
            MessagePayloadType::PublicKey(pk) => {
                let peer_id = self.peer_id;
                if from == peer_id {
                    self.pk_accepted = true;
                }
                let s_slice: &str = &pk[..]; // take a full slice of the string
                let pk: GE = serde_json::from_str(&s_slice)
                    .unwrap_or_else(|_| panic!("Failed to deserialize R"));
                println!("-------Got peer # {:} pk! {:?}", from, pk * &eight_inv);
                self.add_pk(from, pk * &eight_inv);
            }
            _ => panic!("expected public key message"),
        }
    }

    pub fn update_data_step_1(&mut self, from: PeerIdentifier, payload: MessagePayload) {
        let payload_type = EddsaPeer::resolve_payload_type(&payload);
        match payload_type {
            MessagePayloadType::Commitment(t) => {
                println!("-------Got peer # {:} commitment! {:?}", from, t);
                let peer_id = self.peer_id;
                if from == peer_id {
                    self.commitment_accepted = true;
                }
                self.add_commitment(from, t);
            }
            _ => {} //panic!("expected commitment message")
        }
    }

    pub fn update_data_step_2(&mut self, from: PeerIdentifier, payload: MessagePayload) {
        let payload_type = EddsaPeer::resolve_payload_type(&payload);
        match payload_type {
            MessagePayloadType::RMessage(r) => {
                println!("-------Got peer # {:} R message!", from);
                let peer_id = self.peer_id;
                if from == peer_id {
                    self.r_accepted = true;
                }
                self.add_r(from, r);
            }
            _ => {} //panic!("expected R message")
        }
    }

    pub fn update_data_step_3(&mut self, from: PeerIdentifier, payload: MessagePayload) {
        println!("updating data step 3");
        let payload_type = EddsaPeer::resolve_payload_type(&payload);
        match payload_type {
            MessagePayloadType::Signature(s) => {
                println!("-------Got peer # {:} Signature", from);
                let peer_id = self.peer_id;
                if from == peer_id {
                    self.sig_accepted = true;
                }
                self.add_sig(from, s);
            }
            _ => {} //panic!("expected signature message")
        }
    }
}

impl EddsaPeer {
    fn is_step_done(&mut self) -> bool {
        match self.current_step {
            0 => return self.is_done_step_0(),
            1 => return self.is_done_step_1(),
            2 => return self.is_done_step_2(),
            3 => return self.is_done_step_3(),
            _ => panic!("Unsupported step"),
        }
    }
    pub fn is_done_step_0(&self) -> bool {
        self.pks.len() == self.capacity as usize
    }
    pub fn is_done_step_1(&self) -> bool {
        self.commitments.len() == self.capacity as usize
    }
    pub fn is_done_step_2(&self) -> bool {
        self.r_s.len() == self.capacity as usize
    }
    pub fn is_done_step_3(&mut self) -> bool {
        println!("Checking if last step is done");

        if self.sigs.len() == self.capacity as usize {
            self.finalize().unwrap();
            return true;
        }
        false
    }
}

impl EddsaPeer {
    /// steps - in each step the client does a calculation on its
    /// data, and updates the data holder with the new data

    /// step 1 - calculate key and commitment
    pub fn step_1(&mut self) {
        // each peer computes its commitment to the ephemeral key
        // (this implicitly means each party also calculates ephemeral key
        // on this step)
        // round 1: send commitments to ephemeral public keys
        //let mut k = &self.client_key;
        let (ephemeral_key, sign_first_message, sign_second_message) =
            Signature::create_ephemeral_key_and_commit(&self.client_key, &self.message[..]);

        self.ephemeral_key = Some(ephemeral_key);
        // save the commitment
        let _peer_id = self.peer_id;
        match serde_json::to_string(&sign_first_message) {
            Ok(json_string) => {
                //                self.add_commitment(peer_id, json_string.clone());
                let r = serde_json::to_string(&sign_second_message).expect("couldn't create R");
                self.commitment_msg = Some(generate_commitment_message_payload(&json_string));
                self.r_msg = Some(generate_R_message_payload(&r));
            }
            Err(_) => panic!("Couldn't serialize commitment"),
        }
    }

    /// step 2 - return the clients R. No extra calculations
    pub fn step_2(&mut self) {
        println!("Step 2 - no calculations required. Relevant values should be ready");
    }
    /// step 3 - after validating all commitments:
    /// 1. compute APK
    /// 2. compute R' = sum(Ri)
    /// 3. sign message
    pub fn step_3(&mut self) {
        if !self.validate_commitments() {
            // commitments sent by others are not valid. exit
            panic!("Commitments not valid!")
        }
        let agg_key = self.aggregate_pks();
        println!("computed agg_key");
        let r_tot = self.compute_r_tot();
        println!("computed r_tot");
        //       let eph_key = self.ephemeral_key.clone();
        match self.ephemeral_key {
            Some(ref eph_key) => {
                let k = Signature::k(&r_tot, &agg_key.apk, &self.message[..]);
                let peer_id = self.peer_id;
                let r = self
                    .r_s
                    .get(&peer_id)
                    .unwrap_or_else(|| panic!("Client has No R "))
                    .clone();
                let _r: SignSecondMsg =
                    serde_json::from_str(&r).unwrap_or_else(|_| panic!("Failed to deserialize R"));
                let key = &self.client_key;
                // sign
                let s = Signature::partial_sign(&eph_key.r, key, &k, &agg_key.hash, &r_tot);
                let sig_string = serde_json::to_string(&s).expect("failed to serialize signature");
                self.sig_msg = Some(generate_signature_message_payload(&sig_string));
            }
            None => {}
        }
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
            cmtmnt if cmtmnt == String::from(COMMITMENT_MESSAGE_PREFIX) => {
                return MessagePayloadType::Commitment(msg_payload);
            }
            r if r == String::from(R_KEY_MESSAGE_PREFIX) => {
                return MessagePayloadType::RMessage(msg_payload);
            }
            sig if sig == String::from(SIGNATURE_MESSAGE_PREFIX) => {
                return MessagePayloadType::Signature(msg_payload);
            }
            _ => panic!("Unknown relay message prefix"),
        }
    }
}

impl Peer for EddsaPeer {
    fn new(capacity: u32, _message: Vec<u8>, index: u32) -> EddsaPeer {
        println!("Index is {:?}", index);
        let data = fs::read_to_string(format!("keys{}", index))
            .expect("Unable to load keys, did you run keygen first? ");
        let (key, _apk, kg_index): (KeyPair, KeyAgg, u32) = serde_json::from_str(&data).unwrap();
        EddsaPeer {
            client_key: { key },
            pks: HashMap::new(),
            commitments: HashMap::new(),
            r_s: HashMap::new(),
            sigs: HashMap::new(),
            capacity,
            message: _message,
            peer_id: 0,
            agg_key: None,
            kg_index,
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

    fn current_step(&self) -> u32 {
        self.current_step
    }

    fn do_step(&mut self) {
        println!("Current step is: {:}", self.current_step);
        if self.is_step_done() {
            // do the next step
            println!("step {:} done!", self.current_step);
            self.current_step += 1;
            match self.current_step {
                1 => self.step_1(),
                2 => self.step_2(),
                3 => self.step_3(),
                4 => {
                    println!("----------\nDone.\n----------");
                    self.is_done = true;
                }
                _ => panic!("Unsupported step"),
            }
        } else {
            println!("step not done");
        }
    }

    fn update_data(&mut self, from: PeerIdentifier, payload: MessagePayload) {
        // update data according to step
        match self.current_step {
            0 => self.update_data_step_0(from, payload),
            1 => self.update_data_step_1(from, payload),
            2 => self.update_data_step_2(from, payload),
            3 => self.update_data_step_3(from, payload),
            _ => panic!("Unsupported step"),
        }
    }
    /// Does the final calculation of the protocol
    /// in this case:
    ///     collection all signatures
    ///     and verifying the message
    #[allow(non_snake_case)]
    fn finalize(&mut self) -> Result<(), &'static str> {
        let mut s: Vec<Signature> = Vec::new();
        let eight: FE = ECScalar::from(&BigInt::from(8));
        let eight_inv = eight.invert();
        for sig in self.sigs.values() {
            let signature: Signature =
                serde_json::from_str(&sig).expect("Could not serialize signature!");
            s.push(Signature {
                R: signature.R * eight_inv,
                s: signature.s * &eight,
            })
        }
        let signature = Signature::add_signature_parts(s);
        // verify message with signature
        let apk = self.aggregate_pks();

        let data = fs::read_to_string(format!("keys{}", self.peer_id))
            .expect("Unable to load keys, did you run keygen first? ");
        let (_key, orig_apk, _kg_index): (KeyPair, KeyAgg, u32) =
            serde_json::from_str(&data).unwrap();

        let eight: FE = ECScalar::from(&BigInt::from(8));
        let eight_inv = eight.invert();

        let orig_apk = orig_apk.apk * &eight_inv;

        println!("Aggregated pk {:?}", apk);
        println!("Orig pk {:?}", orig_apk);
        // Original apk should be equal to the apk created during signing
        assert_eq!(orig_apk, apk.apk);
        // Verify signature against the original! pubkey
        match verify(&signature, &self.message[..], &orig_apk) {
            Ok(_) => {
                let mut R_vec = signature.R.pk_to_key_slice().to_vec();
                let mut s_vec = BigInt::to_vec(&signature.s.to_big_int());
                s_vec.reverse();
                R_vec.extend_from_slice(&s_vec[..]);

                fs::write(
                    format!("signature{}", self.peer_id),
                    BigInt::from(&R_vec[..]).to_str_radix(16),
                )
                .expect("Unable to save !");
                Ok(())
            }
            Err(_) => Err("Failed to verify"),
        }
    }
    /// check that the protocol is done
    /// and that this peer can finalize its calculations
    fn is_done(&mut self) -> bool {
        self.is_done_step_3()
    }

    /// get the next item the peer needs to send
    /// depending on the current step and the last message
    /// of the peer that was accepted by the server
    fn get_next_item(&mut self) -> Option<MessagePayload> {
        //println!("current_step: {:}, pk_accepted: {:} commitment_accepted: {:} r_accepted: {:} sig_accepted: {:}",self.current_step,self.pk_accepted,self.commitment_accepted, self.r_accepted, self.sig_accepted);
        if self.current_step == 0 || !self.pk_accepted {
            println!("next item is pk: {:?}", self.pk_msg);
            return self.pk_msg.clone();
        }
        if self.current_step == 1 || !self.commitment_accepted {
            println!("next item is commitment: {:?}", self.commitment_msg);
            return self.commitment_msg.clone();
        }
        if self.current_step == 2 || !self.r_accepted {
            println!("next item is r: {:?}", self.r_msg);
            return self.r_msg.clone();
        }
        if self.current_step == 3 || !self.sig_accepted {
            println!("next item is Signature: {:?}", self.sig_msg);
            return self.sig_msg.clone();
        }
        None
    }
}
pub trait Peer {
    fn new(capacity: u32, _message: Vec<u8>, index: u32) -> Self;
    fn zero_step(&mut self, peer_id: PeerIdentifier) -> Option<MessagePayload>;
    fn current_step(&self) -> u32;
    fn do_step(&mut self);
    fn update_data(&mut self, from: PeerIdentifier, payload: MessagePayload);
    fn get_next_item(&mut self) -> Option<MessagePayload>;
    fn finalize(&mut self) -> Result<(), &'static str>;
    fn is_done(&mut self) -> bool;
}

struct ProtocolDataManager<T: Peer> {
    pub peer_id: PeerIdentifier,
    pub capacity: u32,
    pub data_holder: T, // will be filled when initializing, and on each new step
    pub client_data: Option<MessagePayload>, // new data calculated by this peer at the beginning of a step (that needs to be sent to other peers)
    pub new_client_data: bool,
}

impl<T: Peer> ProtocolDataManager<T> {
    pub fn new(capacity: u32, message: Vec<u8>, index: u32) -> ProtocolDataManager<T>
    where
        T: Peer,
    {
        ProtocolDataManager {
            peer_id: 0,
            capacity,
            data_holder: Peer::new(capacity, message, index),
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
            Arg::with_name("message")
                .default_value("message")
                .long("message")
                .short("M"),
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
        client_index: u32,
        capacity: u32,
        message: Vec<u8>,
    ) -> SessionClient {
        let protocol_id = 1;
        SessionClient {
            state: State::new(protocol_id, capacity, client_addr, client_index, message),
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
    pub last_message: RefCell<ClientMessage>,
    pub bc_dests: Vec<ProtocolIdentifier>,
    pub stored_messages: StoredMessages,
    pub timeout: u32,
}

impl<T: Peer> State<T> {
    pub fn new(
        protocol_id: ProtocolIdentifier,
        capacity: u32,
        client_addr: SocketAddr,
        client_index: u32,
        message: Vec<u8>,
    ) -> State<T>
    where
        T: Peer,
    {
        let data_m: ProtocolDataManager<T> =
            ProtocolDataManager::new(capacity, message, client_index);
        State {
            registered: false,
            protocol_id,
            client_addr,
            last_message: RefCell::new(ClientMessage::new()),
            bc_dests: (1..(capacity + 1)).collect(),
            timeout: 100, // 3 second delay in sending messages
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
            println!("-------self message accepted ------\n ");
        }
        let payload = relay_msg.message;
        self.data_manager.get_next_message(from, payload)
    }

    fn generate_relay_message(&self, payload: MessagePayload) -> ClientMessage {
        let _msg = ClientMessage::new();
        // create relay message
        let mut relay_message = RelayMessage::new(
            self.data_manager.peer_id,
            self.protocol_id.clone(),
            self.client_addr,
        );
        let to: Vec<u32> = self.bc_dests.clone();

        let mut client_message = ClientMessage::new();

        relay_message.set_message_params(to, String::from(payload));
        client_message.relay_message = Some(relay_message);
        client_message
    }

    fn handle_register_response(&mut self, peer_id: PeerIdentifier) -> Result<ClientMessage, ()> {
        println!("Peer identifier: {}", peer_id);
        // Set the session parameters
        let message = self
            .data_manager
            .initialize_data(peer_id)
            .unwrap_or_else(|| panic!("failed to initialize"));
        Ok(self.generate_relay_message(message.clone()))
    }

    fn get_last_message(&self) -> Option<ClientMessage> {
        let last_msg = self.last_message.clone().into_inner();
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
                println!("Not initialized, sending again");
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
            _ => {
                println!("didn't handle error correctly");
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
                        println!("sending peers first message: {:#?}", _msg);
                        return Ok(_msg.clone());
                    }
                    Err(_) => {
                        println!("error occured");
                        return Ok(ClientMessage::new());
                    }
                }
            }
            ServerResponse::ErrorResponse(err_msg) => {
                //  println!("got error response");
                let err_msg_slice: &str = &err_msg[..];
                let msg = self.handle_error_response(err_msg_slice);
                match msg {
                    Ok(_msg) => return Ok(_msg),
                    Err(_) => {
                        println!("error occured");
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
        // Try to register with your index on keygen
        msg.register(
            client_addr,
            self.state.protocol_id,
            capacity,
            // FIXME: state should not be that complicated
            self.state.data_manager.data_holder.kg_index as i32,
        );

        println!("Regsiter message {:?}", msg);
        let tx =
            tendermint::abci::transaction::Transaction::new(serde_json::to_string(&msg).unwrap());
        let response = self.client.broadcast_tx_commit(tx).unwrap();
        let server_response = response.clone().deliver_tx.log.unwrap();
        println!("Registered OK");
        println!("ServerResponse {:?}", server_response);
        let server_response: ServerMessage =
            serde_json::from_str(&response.deliver_tx.log.unwrap().to_string()).unwrap();
        println!("ServerResponse {:?}", server_response);
        // TODO Add Error checks etc
        self.state.registered = true;
        return server_response;
    }

    pub fn send_message(&self, msg: ClientMessage) -> BTreeMap<u32, ClientMessage> {
        println!("Sending message {:?}", msg);
        let tx =
            tendermint::abci::transaction::Transaction::new(serde_json::to_string(&msg).unwrap());
        let response = self.client.broadcast_tx_commit(tx).unwrap();
        let server_response = response.clone().deliver_tx.log.unwrap();
        println!("ServerResponse {:?}", server_response);
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

    pub fn handle_relay_message(&mut self, client_msg: ClientMessage) -> Option<ClientMessage> {
        let msg = client_msg.relay_message.unwrap();
        let mut new_message = Some(ClientMessage::new());
        let next = self.state.handle_relay_message(msg.clone());
        println!("Next {:?}", next);
        match next {
            Some(next_msg) => {
                println!("next message to send is {:}", next_msg);
                new_message = Some(self.state.generate_relay_message(next_msg.clone()));
            }
            None => {
                println!("next item is None. Client is finished.");
                new_message = Some(ClientMessage::new());
            }
        }
        new_message
    }

    pub fn generate_client_answer(&mut self, msg: ServerMessage) -> Option<ClientMessage> {
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
                        println!("Error in handle_server_response");
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
            //             //println!("next message to send is {:}", next_msg);
            //             new_message = Some(self.state.generate_relay_message(next_msg.clone()));
            //         }
            //         None => {
            //             println!("next item is None. Client is finished.");
            //             new_message = Some(ClientMessage::new());
            //         }
            //     }
            // }
            ServerMessageType::Abort => {
                println!("Got abort message");
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
    Commitment(String),
    RMessage(String),
    Signature(String),
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

    let message: String = matches
        .value_of("message")
        .unwrap()
        .parse()
        .expect("Invalid message to sign");

    let proxy: String = matches
        .value_of("proxy")
        .unwrap()
        .parse()
        .expect("Invalid proxy address");

    let message_to_sign = match hex::decode(message.to_owned()) {
        Ok(x) => x,
        Err(_) => message.as_bytes().to_vec(),
    };

    // Port and ip address are used as a unique indetifier to the server
    // This should be replaced with PKi down the road
    let port = 8080 + index;
    let proxy_addr = format!("tcp://{}", proxy);
    let client_addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let mut session = SessionClient::new(
        client_addr,
        // TODO: pass tendermint node address as parameter
        &proxy_addr.parse().unwrap(),
        index,
        capacity,
        message_to_sign,
    );
    let server_response = session.register(index, capacity);
    let mut next_message = session.generate_client_answer(server_response);
    println!("Next message: {:?}", next_message);
    // TODO The client/server response could be an error
    let mut server_response = session.send_message(next_message.clone().unwrap());
    session.store_server_response(&server_response);
    // Number of rounds in signing
    let rounds = 4;
    'outer: for _ in 0..rounds {
        'inner: loop {
            let round = session.state.data_manager.data_holder.current_step();
            if session.state.stored_messages.get_number_messages(round) == capacity as usize {
                for msg in session
                    .state
                    .stored_messages
                    .get_messages_vector_client_message(round)
                {
                    next_message = session.handle_relay_message(msg.clone());
                }
                // Do not send response on last round
                if round != rounds - 1 {
                    server_response = session.send_message(next_message.clone().unwrap());
                    session.store_server_response(&server_response);
                }
                break 'inner;
            } else {
                let server_response = session.query();
                // println!("Server response {:?}", server_response);
                // println!("Server response len {}", server_response.keys().len());
                session.store_server_response(&server_response);
                thread::sleep(time::Duration::from_millis(100));
                // println!("All stored messages {:?}", session.state.stored_messages);
            }
        }
    }
}
