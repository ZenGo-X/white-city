use std::collections::HashMap;
use std::fs;

use curv::arithmetic::traits::Converter;
use curv::elliptic::curves::ed25519::*;
use curv::elliptic::curves::traits::ECPoint;
use curv::elliptic::curves::traits::ECScalar;
use curv::{BigInt, FE, GE};
use log::{debug, info};
use multi_party_eddsa::protocols::aggsig::{
    test_com, verify, EphemeralKey, KeyAgg, KeyPair, SignFirstMsg, SignSecondMsg, Signature,
};

use crate::peer::Peer;
use mmpc_server_common::common::*;
use mmpc_server_common::{MessagePayload, PeerIdentifier};

#[derive(Debug)]
pub enum MessagePayloadType {
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
        debug!("Public keys {:?}", &pks);
        debug!("KG index:{}, SIG index:{}", self.kg_index, peer_id);
        // TODO: sort the pks according to key-gen indexes when applying
        KeyPair::key_aggregation_n(&pks, &index)
    }

    fn validate_commitments(&mut self) -> bool {
        // iterate over all peer Rs
        debug!("----------\nvalidating commitments\n----------");
        let eight: FE = ECScalar::from(&BigInt::from(8));
        let eight_inv = eight.invert();
        let r_s = &self.r_s;
        for (peer_id, r) in r_s {
            debug!("peer: {:}", peer_id);
            debug!("r: {:}", r);
            // convert the json_string to a construct
            let _r: SignSecondMsg = serde_json::from_str(r).unwrap();

            // get the corresponding commitment
            let k = peer_id.clone();
            let cmtmnt = self
                .commitments
                .get(&k)
                .expect("peer didn't send commitment");
            debug!("commitment : {:?}", cmtmnt);
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
        debug!("----------\ncommitments valid\n----------");
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
                info!("-------Got peer # {:} pk! {:?}", from, pk * &eight_inv);
                self.add_pk(from, pk * &eight_inv);
            }
            _ => panic!("expected public key message"),
        }
    }

    pub fn update_data_step_1(&mut self, from: PeerIdentifier, payload: MessagePayload) {
        let payload_type = EddsaPeer::resolve_payload_type(&payload);
        match payload_type {
            MessagePayloadType::Commitment(t) => {
                info!("-------Got peer # {:} commitment! {:?}", from, t);
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
                info!("-------Got peer # {:} R message!", from);
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
        debug!("updating data step 3");
        let payload_type = EddsaPeer::resolve_payload_type(&payload);
        match payload_type {
            MessagePayloadType::Signature(s) => {
                debug!("-------Got peer # {:} Signature", from);
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
        debug!("Checking if last step is done");

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
        debug!("Step 2 - no calculations required. Relevant values should be ready");
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
        debug!("computed agg_key");
        let r_tot = self.compute_r_tot();
        debug!("computed r_tot");
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
        debug!("Index is {:?}", index);
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

    fn set_peer_id(&mut self, peer_id: PeerIdentifier) {
        self.peer_id = peer_id;
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

    fn capacity(&self) -> u32 {
        self.capacity
    }

    fn peer_id(&self) -> PeerIdentifier {
        self.peer_id
    }

    fn do_step(&mut self) {
        info!("Current step is: {:}", self.current_step);
        if self.is_step_done() {
            // do the next step
            info!("step {:} done!", self.current_step);
            self.current_step += 1;
            match self.current_step {
                1 => self.step_1(),
                2 => self.step_2(),
                3 => self.step_3(),
                4 => {
                    info!("----------\nDone.\n----------");
                    self.is_done = true;
                }
                _ => panic!("Unsupported step"),
            }
        } else {
            info!("step not done");
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

        debug!("Aggregated pk {:?}", apk);
        debug!("Orig pk {:?}", orig_apk);
        // Original apk should be equal to the apk created during signing
        assert_eq!(orig_apk, apk.apk);
        //assert_eq!(apk, apk.apk);
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
        if self.current_step == 0 || !self.pk_accepted {
            info!("next item is pk: {:?}", self.pk_msg);
            return self.pk_msg.clone();
        }
        if self.current_step == 1 || !self.commitment_accepted {
            info!("next item is commitment: {:?}", self.commitment_msg);
            return self.commitment_msg.clone();
        }
        if self.current_step == 2 || !self.r_accepted {
            info!("next item is r: {:?}", self.r_msg);
            return self.r_msg.clone();
        }
        if self.current_step == 3 || !self.sig_accepted {
            info!("next item is Signature: {:?}", self.sig_msg);
            return self.sig_msg.clone();
        }
        None
    }
}
