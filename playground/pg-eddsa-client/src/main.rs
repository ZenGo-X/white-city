#![allow(non_snake_case)]

extern crate curv;
/// to run:
/// 1: go to rocket_server -> cargo run
/// 2: cargo run from PARTIES number of terminals
extern crate multi_party_ed25519;
extern crate reqwest;
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate serde_json;

use curv::elliptic::curves::traits::ECScalar;
use curv::{BigInt, FE, GE};
use multi_party_ed25519::protocols::aggsig::*;
use reqwest::Client;
use std::env;
use std::fmt;
use std::time::Duration;
use std::{thread, time};

const PARTIES: u32 = 4;

#[derive(Hash, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct TupleKey {
    pub first: String,
    pub second: String,
    pub third: String,
}
impl TupleKey {
    fn new(first: String, second: String, third: String) -> TupleKey {
        return TupleKey {
            first,
            second,
            third,
        };
    }
}
fn pr<T: std::fmt::Debug + ?Sized>(x: &String) {
    println!("{:?}", &*x);
}
impl fmt::Display for TupleKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.first, self.second, self.third)
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PartySignup {
    pub number: u32,
    pub uuid: String,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Index {
    pub key: TupleKey,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub key: TupleKey,
    pub value: String,
}

fn main() {
    // working with ed25519 communication we make sure we are in the prime sub group
    let eight_bn = BigInt::from(8);
    let eight: FE = ECScalar::from(&eight_bn);
    let eight_inv = eight.invert();
    // delay:
    let ten_millis = time::Duration::from_millis(10);

    let message: [u8; 4] = [79, 77, 69, 82]; //TODO: make arg
    let client = Client::new();

    let party_i_signup_result = signup(&client);

    assert!(party_i_signup_result.is_ok());
    let party_i_signup = party_i_signup_result.unwrap();
    println!("{:?}", party_i_signup.clone());

    let party_num_int = party_i_signup.number.clone();
    let uuid = party_i_signup.uuid;
    //////////////////////////////////////////////////////////////////////////////

    let party_key = KeyPair::create();
    let (party_ephemeral_key, sign_first_message, sign_second_message) =
        Signature::create_ephemeral_key_and_commit(&party_key, &message);
    //////////////////////////////////////////////////////////////////////////////

    //round 0: send public key, get public keys from other parties (the protocol is not specifying anything on how to share pubkeys)
    assert!(send(
        &client,
        party_num_int.clone(),
        "round0",
        serde_json::to_string(&party_key.public_key).unwrap(),
        uuid.clone()
    )
    .is_ok());
    let round0_ans_vec = poll_for_peers(
        &client,
        party_num_int.clone(),
        PARTIES,
        ten_millis.clone(),
        "round0",
        uuid.clone(),
    );

    //////////////////////////////////////////////////////////////////////////////
    //compute apk:
    let mut j = 0;
    let mut pks: Vec<GE> = Vec::new();
    for i in 1..PARTIES + 1 {
        if i == party_num_int {
            pks.push(&party_key.public_key * &eight);
        } else {
            let party_i_pubkey: GE = serde_json::from_str(&round0_ans_vec[j]).unwrap();
            pks.push(party_i_pubkey);
            j = j + 1;
        }
    }
    let partyi_key_agg = KeyPair::key_aggregation_n(&pks, &(party_num_int as usize - 1));
    //////////////////////////////////////////////////////////////////////////////

    // send commitment to ephemeral public keys, get round 1 commitments of other parties
    assert!(send(
        &client,
        party_num_int.clone(),
        "round1",
        serde_json::to_string(&sign_first_message).unwrap(),
        uuid.clone()
    )
    .is_ok());
    let round1_ans_vec = poll_for_peers(
        &client,
        party_num_int.clone(),
        PARTIES,
        ten_millis.clone(),
        "round1",
        uuid.clone(),
    );

    // round 2: send ephemeral public keys and  check commitments correctness
    assert!(send(
        &client,
        party_num_int.clone(),
        "round2",
        serde_json::to_string(&sign_second_message).unwrap(),
        uuid.clone()
    )
    .is_ok());
    let round2_ans_vec = poll_for_peers(
        &client,
        party_num_int.clone(),
        PARTIES,
        ten_millis.clone(),
        "round2",
        uuid.clone(),
    );

    //////////////////////////////////////////////////////////////////////////////
    // test commitments and construct R
    let mut Ri: Vec<GE> = Vec::new();
    let mut j = 0;
    for i in 1..PARTIES + 1 {
        if i != party_num_int {
            let party_i_first_message: SignFirstMsg =
                serde_json::from_str(&round1_ans_vec[j]).unwrap();
            let party_i_second_message: SignSecondMsg =
                serde_json::from_str(&round2_ans_vec[j]).unwrap();
            let R_inv_eight = &party_i_second_message.R * &eight_inv;
            assert!(test_com(
                &R_inv_eight,
                &party_i_second_message.blind_factor,
                &party_i_first_message.commitment
            ));
            Ri.push(party_i_second_message.R);
            println!("party {:?} comm is valid", i);
            j = j + 1;
        } else {
            Ri.push(&party_ephemeral_key.R * &eight);
        }
    }
    // calculate local signature:
    let R_tot = Signature::get_R_tot(Ri);
    let k = Signature::k(&R_tot, &partyi_key_agg.apk, &message);
    let si = Signature::partial_sign(
        &party_ephemeral_key.r,
        &party_key,
        &k,
        &partyi_key_agg.hash,
        &R_tot,
    );

    //////////////////////////////////////////////////////////////////////////////

    // round 3: send ephemeral public keys and  check commitments correctness
    assert!(send(
        &client,
        party_num_int.clone(),
        "round3",
        serde_json::to_string(&si).unwrap(),
        uuid.clone()
    )
    .is_ok());
    let round3_ans_vec = poll_for_peers(
        &client,
        party_num_int.clone(),
        PARTIES,
        ten_millis.clone(),
        "round3",
        uuid.clone(),
    );

    //////////////////////////////////////////////////////////////////////////////

    // compute signature:
    let mut j = 0;
    let mut s: Vec<Signature> = Vec::new();
    for i in 1..PARTIES + 1 {
        if i == party_num_int {
            s.push(Signature {
                R: R_tot.clone(),
                s: si.s.clone() * &eight,
            });
        } else {
            let party_i_si: Signature = serde_json::from_str(&round3_ans_vec[j]).unwrap();
            let party_i_si = Signature {
                R: R_tot.clone(),
                s: party_i_si.s * &eight,
            }; //same R for all partial sigs. TODO: send only s part of the partial sigs?
            s.push(party_i_si);
            j = j + 1;
        }
    }

    let signature = Signature::add_signature_parts(s);
    assert!(verify(&signature, &message, &partyi_key_agg.apk).is_ok());
    println!(" {:?} \n on message : {:?}", signature, message);
    //////////////////////////////////////////////////////////////////////////////
}

pub fn postb<T>(client: &Client, path: &str, body: T) -> Option<String>
where
    T: serde::ser::Serialize,
{
    let res = client
        .post(&format!("http://127.0.0.1:8001/{}", path))
        .json(&body)
        .send();
    Some(res.unwrap().text().unwrap())
}

pub fn signup(client: &Client) -> Result<(PartySignup), ()> {
    let key = TupleKey {
        first: "signup".to_string(),
        second: "".to_string(),
        third: "".to_string(),
    };

    let res_body = postb(&client, "signup", key).unwrap();
    let answer: Result<(PartySignup), ()> = serde_json::from_str(&res_body).unwrap();
    return answer;
}

pub fn send(
    client: &Client,
    party_num: u32,
    round: &str,
    data: String,
    uuid: String,
) -> Result<(), ()> {
    let key = TupleKey {
        first: party_num.to_string(),
        second: round.to_string(),
        third: uuid,
    };
    let entry = Entry {
        key: key.clone(),
        value: data,
    };

    let res_body = postb(&client, "set", entry).unwrap();
    let answer: Result<(), ()> = serde_json::from_str(&res_body).unwrap();
    return answer;
}

pub fn poll_for_peers(
    client: &Client,
    party_num: u32,
    n: u32,
    delay: Duration,
    round: &str,
    uuid: String,
) -> Vec<String> {
    let mut ans_vec = Vec::new();
    for i in 1..n + 1 {
        if i != party_num {
            let key = TupleKey {
                first: i.to_string(),
                second: round.to_string(),
                third: uuid.clone(),
            };
            let index = Index { key };
            loop {
                // add delay to allow the server to process request:
                thread::sleep(delay);
                let res_body = postb(client, "get", index.clone()).unwrap();
                let answer: Result<Entry, ()> = serde_json::from_str(&res_body).unwrap();
                if answer.is_ok() {
                    ans_vec.push(answer.unwrap().value);
                    println!("party {:?} {:?} read success", i, round);
                    break;
                }
            }
        }
    }
    ans_vec
}
