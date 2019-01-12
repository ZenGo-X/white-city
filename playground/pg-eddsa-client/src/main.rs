
extern crate multi_party_ed25519;
extern crate curv;
extern crate reqwest;
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate serde_json;

use std::fmt;
use std::{thread, time};
use multi_party_ed25519::protocols::aggsig::*;
use std::env;
use reqwest::Client;
use curv::{BigInt,FE,GE};
use curv::elliptic::curves::traits::ECScalar;
use std::time::Duration;

#[derive(Hash)]
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct TupleKey {
    pub first: String,
    pub second: String
}
impl TupleKey {
    fn new(first: String, second: String) -> TupleKey {
        return TupleKey {first, second };
    }
}
fn pr<T : std::fmt::Debug + ?Sized>(x: &String) {
    println!("{:?}", &*x);
}
impl fmt::Display for TupleKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.first, self.second)
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Index{
    pub key: TupleKey,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Entry{
    pub key: TupleKey,
    pub value: String,
}


fn main(){
    // working with ed25519 communication we make sure we are in the prime sub group
    let eight_bn = BigInt::from(8);
    let eight: FE  = ECScalar::from(&eight_bn);
    let eight_inv = eight.invert();
    // delay:
    let ten_millis = time::Duration::from_millis(10);
    let party_num_string = env::args().nth(1).unwrap();
    let party_num_int: i32 = party_num_string.parse().unwrap();
    let n_string = env::args().nth(2).unwrap();
    let n: i32 = n_string.parse().unwrap();
    let message: [u8; 4] = [79, 77, 69, 82]; //TODO: make arg
    let client = Client::new();

    let party1_key = KeyPair::create();
    let (party1_ephemeral_key, sign_first_message, sign_second_message) =
        Signature::create_ephemeral_key_and_commit(&party1_key, &message);

    assert!(test_com(
        &sign_second_message.R,
        &sign_second_message.blind_factor,
        &sign_first_message.commitment
    ));
    // send commitment to ephemeral public keys

    let key  = TupleKey{
        first: party_num_string.clone(),
        second: "round1".to_string(),
    };
    let entry = Entry{
        key: key.clone(),
        value: serde_json::to_string(&sign_first_message).unwrap(),
    };


    let res_body = postb(&client, "set", entry).unwrap();
    let answer : Result<(),()> = serde_json::from_str(&res_body).unwrap();
    assert!(answer.is_ok());

    // get round 1 commitments of other parties
    let round1_ans_vec = poll_for_peers(&client, party_num_int.clone(), n.clone(), ten_millis.clone(),"round1" );

    // round 2: send ephemeral public keys and  check commitments correctness
    let key  = TupleKey{
        first: party_num_string.clone(),
        second: "round2".to_string(),
    };
    let entry = Entry{
        key: key.clone(),
        value: serde_json::to_string(&sign_second_message).unwrap(),
    };


    let res_body = postb(&client, "set", entry).unwrap();
    let answer : Result<(),()> = serde_json::from_str(&res_body).unwrap();
    assert!(answer.is_ok());

    let round2_ans_vec = poll_for_peers(&client, party_num_int.clone(), n.clone(), ten_millis.clone(),"round2" );

    // test commitments
    let mut j = 0;
    for i in 1..n+1 {
        if i != party_num_int{
            let party_i_first_message : SignFirstMsg = serde_json::from_str(&round1_ans_vec[j]).unwrap();
            let party_i_second_message : SignSecondMsg = serde_json::from_str(&round2_ans_vec[j]).unwrap();
            let R_inv_eight = &party_i_second_message.R * &eight_inv;
            assert!(test_com(
                &R_inv_eight,
                &party_i_second_message.blind_factor,
                &party_i_first_message.commitment
            ));
            println!("party {:?} comm is valid", i);
            j = j + 1;
        }
    }

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

pub fn poll_for_peers(client: &Client, party_num: i32, n: i32,  delay: Duration, round: &str) -> Vec<String> {
    let mut ans_vec = Vec::new();
    for i in 1..n+1 {

        if i != party_num {
            let key = TupleKey {
                first: i.to_string(),
                second: round.to_string(),
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