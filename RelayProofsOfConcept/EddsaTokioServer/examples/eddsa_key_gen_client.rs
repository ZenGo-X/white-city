#![feature(refcell_replace_swap)]
extern crate chrono;
extern crate dict;
///
/// Implementation of a client that communicates with the relay server
/// This client represents eddsa peer
///
///
extern crate futures;
extern crate relay_server_common;
extern crate structopt;
extern crate tokio_core;

use chrono::prelude::*;

use std::cell::RefCell;
use std::env;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::vec::Vec;
use std::{thread, time};
use structopt::StructOpt;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use tokio_core::io::Io;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Core;

use futures::stream;
use futures::sync::mpsc;
use futures::{Future, Sink, Stream};

use relay_server_common::{
    ClientMessage, ClientToServerCodec, MessagePayload, PeerIdentifier, ProtocolIdentifier,
    RelayMessage, ServerMessage, ServerMessageType, ServerResponse,
};

// unique to our eddsa client
extern crate curv;
extern crate multi_party_ed25519;

use curv::elliptic::curves::ed25519::*;
use curv::elliptic::curves::traits::ECPoint;
use curv::elliptic::curves::traits::ECScalar;
use curv::{BigInt, FE, GE};

use multi_party_ed25519::protocols::aggsig::{
    test_com, verify, EphemeralKey, KeyAgg, KeyPair, SignFirstMsg, SignSecondMsg, Signature,
};
//use multi_party_ed25519::

use relay_server_common::common::*;

use dict::{Dict, DictIface};
use std::collections::HashMap;
use std::fs;

// Arguments parsing
#[derive(StructOpt, Debug)]
#[structopt(name = "eddsa-key-get-client")]
struct Opt {
    /// Number of participants in the protocol
    #[structopt(short = "P", long = "participants", default_value = "2")]
    capacity: u32,

    /// Address the server listens on
    #[structopt(name = "ADDRESS")]
    address: String,

    /// Output file
    #[structopt(name = "KEY_FILE", parse(from_os_str))]
    output: PathBuf,
}

struct EddsaPeer {
    // this peers identifier in this session
    pub peer_id: RefCell<PeerIdentifier>,
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
    pub message: &'static [u8],

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
        let mut Ri: Vec<GE> = Vec::new();
        for (peer_id, r) in &self.r_s {
            let r_slice: &str = &r[..];
            let _r: SignSecondMsg =
                serde_json::from_str(r_slice).unwrap_or_else(|e| panic!("serialization error"));
            Ri.push(_r.R.clone());
        }
        let r_tot = Signature::get_R_tot(Ri);
        return r_tot;
    }
    fn aggregate_pks(&mut self) -> KeyAgg {
        println!("aggregating pks");
        let cap = self.capacity as usize;
        let mut pks = Vec::with_capacity(self.capacity as usize);
        for index in 0..self.capacity {
            let peer = index + 1;
            let mut pk = self.pks.get_mut(&peer).unwrap(); //_or_else(||{println!("dont have peers pk");});
            pks.push(pk.clone());
        }
        println!("# of public keys : {:?}", pks.len());
        let peer_id = self.peer_id.clone().into_inner();
        let index = (peer_id - 1) as usize;
        let agg_key = KeyPair::key_aggregation_n(&pks, &index);
        return agg_key;
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
        match payload_type {
            MessagePayloadType::PUBLIC_KEY(pk) => {
                let peer_id = self.peer_id.clone().into_inner();
                if from == peer_id {
                    self.pk_accepted = true;
                }
                let s_slice: &str = &pk[..]; // take a full slice of the string
                let _pk = serde_json::from_str(s_slice);
                println!("-------Got peer # {:} pk! {:?}", from, pk);
                match _pk {
                    Ok(_pk) => self.add_pk(from, _pk),
                    Err(e) => panic!("Could not serialize public key"),
                }
            }
            _ => panic!("expected public key message"),
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
            self.finalize();
            return true;
        }
        false
    }

    /// Check if peer should finalize the session
    pub fn should_finalize(&mut self) -> bool {
        self.is_done()
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
                return MessagePayloadType::PUBLIC_KEY(msg_payload);
            }
            _ => panic!("Unknown relay message prefix"),
        }
    }
}

impl Peer for EddsaPeer {
    fn new(capacity: u32, _message: &'static [u8]) -> EddsaPeer {
        EddsaPeer {
            client_key: KeyPair::create(),
            pks: HashMap::new(),
            commitments: HashMap::new(),
            r_s: HashMap::new(),
            sigs: HashMap::new(),
            capacity,
            message: _message,
            peer_id: RefCell::new(0),
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
        self.peer_id.replace(peer_id);
        let pk/*:Ed25519Point */= self.client_key.public_key.clone();
        //self.add_pk(peer_id, pk);

        let pk_s = serde_json::to_string(&pk).expect("Failed in serialization");

        self.pk_msg = Some(generate_pk_message_payload(&pk_s));
        return self.pk_msg.clone();
    }

    fn do_step(&mut self) {
        println!("Current step is: {:}", self.current_step);
        if self.is_step_done() {
            // do the next step
            println!("step {:} done!", self.current_step);
            self.current_step += 1;
            match self.current_step {
                1 => {
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

        let res = fs::write(env::args().nth(2).unwrap(), keygen_json);
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err("Failed to verify"),
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
            println!("next item is pk: {:?}", self.pk_msg);
            return self.pk_msg.clone();
        }
        None
    }
}
pub trait Peer {
    fn new(capacity: u32, _message: &'static [u8]) -> Self;
    fn zero_step(&mut self, peer_id: PeerIdentifier) -> Option<MessagePayload>;
    fn do_step(&mut self);
    fn update_data(&mut self, from: PeerIdentifier, payload: MessagePayload);
    fn get_next_item(&mut self) -> Option<MessagePayload>;
    fn finalize(&mut self) -> Result<(), &'static str>;
    fn is_done(&mut self) -> bool;
}

struct ProtocolDataManager<T: Peer> {
    pub peer_id: RefCell<PeerIdentifier>,
    pub capacity: u32,
    pub current_step: u32,
    pub data_holder: T, // will be filled when initializing, and on each new step
    pub client_data: Option<MessagePayload>, // new data calculated by this peer at the beginning of a step (that needs to be sent to other peers)
    pub new_client_data: bool,
}

impl<T: Peer> ProtocolDataManager<T> {
    pub fn new(capacity: u32, message: &'static [u8]) -> ProtocolDataManager<T>
    where
        T: Peer,
    {
        ProtocolDataManager {
            peer_id: RefCell::new(0),
            current_step: 0,
            capacity,
            data_holder: Peer::new(capacity, message),
            client_data: None,
            new_client_data: false,
            //message: message.clone(),
        }
    }

    /// set manager with the initial values that a local peer holds at the beginning of
    /// the protocol session
    /// return: first message
    pub fn initialize_data(&mut self, peer_id: PeerIdentifier) -> Option<MessagePayload> {
        self.peer_id.replace(peer_id);
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

struct Client_W<T>(RefCell<Client<T>>)
where
    T: Peer;

struct Client<T>
where
    T: Peer,
{
    pub registered: bool,
    pub protocol_id: ProtocolIdentifier,
    pub data_manager: ProtocolDataManager<T>,
    pub last_message: RefCell<ClientMessage>,
    pub bc_dests: Vec<ProtocolIdentifier>,
    pub timeout: u32,
}
impl<T: Peer> Client<T> {
    pub fn new(protocol_id: ProtocolIdentifier, capacity: u32, message: &'static [u8]) -> Client<T>
    where
        T: Peer,
    {
        let data_m: ProtocolDataManager<T> = ProtocolDataManager::new(capacity, message);
        Client {
            registered: false,
            protocol_id,
            last_message: RefCell::new(ClientMessage::new()),
            bc_dests: (1..(capacity + 1)).collect(),
            timeout: 100, // 3 second delay in sending messages
            data_manager: data_m,
        }
    }

    pub fn generate_client_answer(&mut self, msg: ServerMessage) -> Option<ClientMessage> {
        let last_message = self.last_message.clone().into_inner();
        let mut new_message = None;
        let msg_type = msg.msg_type();
        match msg_type {
            ServerMessageType::Response => {
                let next = self.handle_server_response(&msg);
                match next {
                    Ok(next_msg) => {
                        new_message = Some(next_msg.clone());
                    }
                    Err(e) => {
                        println!("Error in handle_server_response");
                    }
                }
            }
            ServerMessageType::RelayMessage => {
                let next = self.handle_relay_message(msg.clone());
                match next {
                    Some(next_msg) => {
                        //println!("next message to send is {:}", next_msg);
                        new_message = Some(self.generate_relay_message(next_msg.clone()));
                    }
                    None => {
                        println!("next item is None. Client is finished.");
                        new_message = Some(ClientMessage::new());
                    }
                }
            }
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
        if last_message.is_empty() {
            match new_message {
                Some(msg) => {
                    self.last_message.replace(msg.clone());
                    return Some(msg.clone());
                }
                None => return None,
            }
        } else {
            let _new_message = new_message.clone().unwrap();
            if !last_message.are_equal_payloads(&_new_message) {
                println!("last message changed");
                self.last_message.replace(_new_message.clone());
            }
            self.wait_timeout();
            return Some(self.last_message.clone().into_inner());
        }
    }

    pub fn generate_register_message(&mut self) -> ClientMessage {
        let mut msg = ClientMessage::new();
        msg.register(self.protocol_id.clone(), self.data_manager.capacity.clone());
        msg
    }
}

impl<T: Peer> Client<T> {
    fn set_bc_dests(&mut self) {
        //        let index = self.data_manager.peer_id.clone().into_inner() - 1;
        //        self.bc_dests.remove(index as usize);
    }

    fn handle_relay_message(&mut self, msg: ServerMessage) -> Option<MessagePayload> {
        // parse relay message
        let relay_msg = msg.relay_message.unwrap();
        let from = relay_msg.peer_number;
        if from == self.data_manager.peer_id.clone().into_inner() {
            println!("-------self message accepted ------\n ");
        }
        let payload = relay_msg.message;
        self.data_manager.get_next_message(from, payload)
    }

    fn generate_relay_message(&self, payload: MessagePayload) -> ClientMessage {
        let msg = ClientMessage::new();
        // create relay message
        let mut relay_message = RelayMessage::new(
            self.data_manager.peer_id.clone().into_inner(),
            self.protocol_id.clone(),
        );
        let mut to: Vec<u32> = self.bc_dests.clone();

        let mut client_message = ClientMessage::new();

        relay_message.set_message_params(to, String::from(payload));
        client_message.relay_message = Some(relay_message);
        client_message
    }

    fn wait_timeout(&self) {
        //    println!("Waiting timeout..");
        let wait_time = time::Duration::from_millis(self.timeout as u64);
        thread::sleep(wait_time);
    }

    fn handle_register_response(&mut self, peer_id: PeerIdentifier) -> Result<ClientMessage, ()> {
        println!("Peer identifier: {}", peer_id);
        // Set the session parameters
        let message = self
            .data_manager
            .initialize_data(peer_id)
            .unwrap_or_else(|| panic!("failed to initialize"));
        self.set_bc_dests();
        //      self.wait_timeout();
        // self.last_message.replace(self.generate_relay_message(message.clone()));
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
                // wait
                //    self.wait_timeout();
                //              println!("sending again");
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
                    Err(e) => {
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
                    Err(e) => {
                        println!("error occured");
                        return Ok(ClientMessage::new());
                    }
                }
            }
            // ServerResponse::GeneralResponse(msg) => {
            //     unimplemented!()
            //   },
            ServerResponse::NoResponse => unimplemented!(),
            _ => panic!("failed to handle response"),
        }
    }
}

pub enum MessageProcessResult {
    Message,
    NoMessage,
    Abort,
}

#[derive(Debug)]
enum MessagePayloadType {
    /// Types of expected relay messages
    /// for step 0 we expect PUBLIC_KEY_MESSAGE
    /// for step 1 we expect COMMITMENT
    /// for step 2 we expect R_MESSAGE
    /// for step 3 we expect SIGNATURE
    PUBLIC_KEY(String),
    COMMITMENT(String),
    R_MESSAGE(String),
    SIGNATURE(String),
}

static message_to_sign: [u8; 4] = [79, 77, 69, 82];

fn main() {
    let opt = Opt::from_args();

    let addr = opt.address;

    let PROTOCOL_IDENTIFIER_ARG = 1;
    let PROTOCOL_CAPACITY_ARG = opt.capacity;

    let addr = addr.parse::<SocketAddr>().unwrap();

    // Create the event loop and initiate the connection to the remote server
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let _tcp = TcpStream::connect(&addr, &handle);

    let mut count = Arc::new(AtomicUsize::new(0));

    let mut session: Arc<Mutex<Client_W<EddsaPeer>>> =
        Arc::new(Mutex::new(Client_W(RefCell::new(Client::new(
            PROTOCOL_IDENTIFIER_ARG,
            PROTOCOL_CAPACITY_ARG,
            &message_to_sign,
        )))));
    let client = _tcp
        .and_then(|stream| {
            //println!("sending register message");
            let framed_stream = stream.framed(ClientToServerCodec::new());
            let mut session_ = session.lock().unwrap();
            let msg = session_.0.get_mut().generate_register_message();

            // send register message to server
            let send_ = framed_stream.send(msg);
            let session_inner = Arc::clone(&session);
            let count_inner = Arc::clone(&count);
            send_.and_then(|stream| {
                let (tx, rx) = stream.split();
                let client = rx
                    .then(move |msg| {
                        let mut session_i = session_inner.lock().unwrap();
                        let session_inner = session_i.0.get_mut();
                        if msg.is_err() {
                            println!("got error instead of message from server");
                            return Ok(ClientMessage::new());
                        }
                        let result = session_inner.generate_client_answer(msg.unwrap());
                        match result {
                            Some(_msg) => {
                                return Ok(_msg);
                            }
                            None => return Ok(ClientMessage::new()),
                        }
                    })
                    .then(|msg| {
                        match msg {
                            // placeholder for debugging before message forwarding
                            Ok(x) => {
                                //println!("forwarding {:?}",x);
                                Ok(x)
                            }
                            Err(e) => Err(e),
                        }
                    })
                    .forward(tx);
                client

                //            client.map_err(|err|{println!("ERROR CLIENT: {:?}",err);err})
            })
        })
        .map_err(|err| {
            // All tasks must have an `Error` type of `()`. This forces error
            // handling and helps avoid silencing failures.
            //
            // In our example, we are only going to log the error to STDOUT.
            println!("connection error = {:?}", err);
        });

    core.run(client); //.unwrap();
}
