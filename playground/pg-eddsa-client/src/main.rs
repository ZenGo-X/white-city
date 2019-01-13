/// to run:
/// 1: go to rocket_server -> cargo run
/// 2: cargo run [party_id] [num_parties] i.e. cargo run 1 3 to run as party1 in a 3 party protocol
/// 3. in separate terminals run the other parties, i.e. cargo run 2 3 and cargo run 3 3
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

    let party_key = KeyPair::create();
    let (party_ephemeral_key, sign_first_message, sign_second_message) =
        Signature::create_ephemeral_key_and_commit(&party_key, &message);

    //round 0: send public key, get public keys from other parties (the protocol is not specifying any thing on how to share pubkeys)
    assert!(send(&client, party_num_int.clone(), "round0", serde_json::to_string(&party_key.public_key).unwrap()).is_ok());
    let round0_ans_vec = poll_for_peers(&client, party_num_int.clone(), n.clone(), ten_millis.clone(),"round0" );



    //////////////////////////////////////////////////////////////////////////////
    //compute apk:
    let mut j = 0;
    let mut pks: Vec<GE> = Vec::new();
    for i in 1..n+1 {

        if i == party_num_int {
            pks.push(party_key.public_key.clone());
        }
        else{

            let party_i_pubkey : GE = serde_json::from_str(&round0_ans_vec[j]).unwrap();
            let pk_i_inv_eight = party_i_pubkey * &eight_inv;
            pks.push(pk_i_inv_eight);
            j = j + 1;
        }

    }
    let partyi_key_agg = KeyPair::key_aggregation_n(&pks, &(party_num_int as usize - 1));
    //////////////////////////////////////////////////////////////////////////////

    // send commitment to ephemeral public keys, get round 1 commitments of other parties
    assert!(send(&client, party_num_int.clone(), "round1", serde_json::to_string(&sign_first_message).unwrap()).is_ok());
    let round1_ans_vec = poll_for_peers(&client, party_num_int.clone(), n.clone(), ten_millis.clone(),"round1" );

    // round 2: send ephemeral public keys and  check commitments correctness
    assert!(send(&client, party_num_int.clone(), "round2", serde_json::to_string(&sign_second_message).unwrap()).is_ok());
    let round2_ans_vec = poll_for_peers(&client, party_num_int.clone(), n.clone(), ten_millis.clone(),"round2" );

    //////////////////////////////////////////////////////////////////////////////
    // test commitments and construct R
    let mut Ri: Vec<GE> = Vec::new();
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
            Ri.push(R_inv_eight);
            println!("party {:?} comm is valid", i);
            j = j + 1;
        }
        else{
            Ri.push(party_ephemeral_key.R.clone());
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
    assert!(send(&client, party_num_int.clone(), "round3", serde_json::to_string(&si).unwrap()).is_ok());
    let round3_ans_vec = poll_for_peers(&client, party_num_int.clone(), n.clone(), ten_millis.clone(),"round3" );

    //////////////////////////////////////////////////////////////////////////////

    // compute signature:
    let mut j = 0;
    let mut s: Vec<Signature> = Vec::new();
    for i in 1..n+1 {

        if i == party_num_int {
            s.push(si.clone());
        }
            else{
                let party_i_si : Signature = serde_json::from_str(&round3_ans_vec[j]).unwrap();
                let party_i_si = Signature{ R : R_tot.clone(), s: party_i_si.s}; //same R for all partial sigs. TODO: send only s part of the partial sigs?
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

pub fn send(client: &Client, party_num: i32,  round: &str, data: String) -> Result<(),()>{
    let key  = TupleKey{
        first: party_num.to_string(),
        second: round.to_string(),
    };
    let entry = Entry{
        key: key.clone(),
        value: data,
    };

    let res_body = postb(&client, "set", entry).unwrap();
    let answer : Result<(),()> = serde_json::from_str(&res_body).unwrap();
    return answer;
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