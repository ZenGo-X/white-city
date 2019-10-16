use std::collections::HashMap;
use std::fs;

use curv::elliptic::curves::ed25519::*;
use log::{debug, info};
use multi_party_eddsa::protocols::aggsig::{EphemeralKey, KeyAgg, KeyPair};

use crate::peer::{MessagePayloadType, Peer};
use mmpc_server_common::common::*;
use mmpc_server_common::{MessagePayload, PeerIdentifier};

#[allow(non_snake_case)]
pub struct EddsaPeer {
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

impl Peer for EddsaPeer {
    fn new(capacity: u32, _message: Vec<u8>, index: u32) -> EddsaPeer {
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

    fn current_step(&self) -> u32 {
        self.current_step
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
