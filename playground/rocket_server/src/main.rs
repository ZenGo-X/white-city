#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

extern crate rocket_contrib;
extern crate reqwest;
extern crate uuid;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use std::fmt;
use std::str;
use std::collections::HashMap;
use rocket::State;
use rocket_contrib::json::Json;
use std::sync::Mutex;
use std::sync::RwLock;
use rocket::config::{Config, Environment};
use uuid::Uuid;

const PARTIES: u32 = 3;

#[derive(Hash)]
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct TupleKey {
    pub first: String,
    pub second: String,
    pub third: String,
}
impl TupleKey {
    fn new(first: String, second: String, third: String) -> TupleKey {
        return TupleKey {first, second, third };
    }
}
fn pr<T : std::fmt::Debug + ?Sized>(x: &String) {
    println!("{:?}", &*x);
}
impl fmt::Display for TupleKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.first, self.second,  self.third)
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PartySignup {
    pub number: u32,
    pub uuid: String,
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
#[post("/get", format = "json", data = "<request>")]
fn get(
    db_mtx: State<RwLock<HashMap<TupleKey,String>>>,
    request: Json<Index>
) -> Json<Result<Entry,()>> {
    let index: Index = request.0;
    let mut hm = db_mtx.read().unwrap();
    match hm.get(&index.key){
        Some(v) => {
            let entry = Entry{
                key: index.key,
                value: format!("{}", v.clone()),
            };
            Json(Ok(entry))
        },
        None => Json(Err(())),
    }

}

#[post("/set", format = "json", data = "<request>")]
fn set(
    db_mtx: State<RwLock<HashMap<TupleKey,String>>>,
    request: Json<Entry>
) -> Json<Result<(),()>> {
    let entry: Entry = request.0;
    let mut hm = db_mtx.write().unwrap();
    hm.insert(entry.key.clone(), entry.value.clone());
    Json(Ok(()))
}

#[post("/signup", format = "json")]
fn signup(
    db_mtx: State<RwLock<HashMap<TupleKey,String>>>,
) -> Json<Result<PartySignup,()>> {
    let key  = TupleKey{
        first: "signup".to_string(),
        second: "".to_string(),
        third: "".to_string(),
    };
    let mut hm = db_mtx.read().unwrap();
    match hm.get(&key){
        Some(value) => {
            let party_i_minus1_signup : PartySignup = serde_json::from_str(&value).unwrap();
            if party_i_minus1_signup.number < PARTIES {
                let party_num = party_i_minus1_signup.number + 1;
                let party_signup = PartySignup {
                    number: party_num.clone(),
                    uuid: party_i_minus1_signup.uuid,
                };
                // update state:
                drop(hm);
                let mut hm = db_mtx.write().unwrap();
                hm.insert(key, serde_json::to_string(&party_signup).unwrap());
                return Json(Ok(party_signup))
            }
        },
        None => {},
    }
    // start new session

    let uuid = Uuid::new_v4().to_string();
    let party1 = 1;
    let party_signup = PartySignup {
        number: party1,
        uuid,
    };

    // update state:
    drop(hm);
    let mut hm = db_mtx.write().unwrap();
    hm.insert(key, serde_json::to_string(&party_signup).unwrap());

    Json(Ok(party_signup))

}

/*
fn start_new_session(key: TupleKey, db_mtx: State<RwLock<HashMap<TupleKey,String>>>) -> Json<Result<PartySignup,()>> {

    let uuid = Uuid::new_v4().to_string();
    let party1 = 1;
    let party_signup = PartySignup {
        number: party1,
        uuid,
    };
    // update state:
    let mut hm = db_mtx.write().unwrap();
    hm.insert(key, serde_json::to_string(&party_signup).unwrap());
    Json(Ok(party_signup))
}
*/
//refcell, arc

fn main() {


    let mut my_config = Config::development();
   // my_config.set_port(18001);
    let db : HashMap<TupleKey, String> = HashMap::new();
    let db_mtx = RwLock::new(db);
    //rocket::custom(my_config).mount("/", routes![get, set]).manage(db_mtx).launch();
    rocket::ignite().mount("/", routes![get, set, signup]).manage(db_mtx).launch();

}


pub mod tests {
    use reqwest;
    use super::{TupleKey, Entry, Index};
    use serde_json;

    #[test]
    pub fn simple_set_get() {
        let client = reqwest::Client::new();

        let key  = TupleKey{
            first: "omer".to_string(),
            second: "shlomovits".to_string(),
            third: "".to_string(),
        };
        let entry = Entry{
            key: key.clone(),
            value: "secret".to_string(),
        };
        let res_body = postb(&client, "set", entry).unwrap();
        let answer1 : Result<(),()> = serde_json::from_str(&res_body).unwrap();
        println!("answer1: {:?}", answer1);


        let index = Index{key};
        let res_body = postb(&client, "get", index).unwrap();
        let answer2 : Result<Entry,()> = serde_json::from_str(&res_body).unwrap();
        println!("answer2: {:?}", answer2);
    }

    pub fn postb<T>(client: &reqwest::Client, path: &str, body: T) -> Option<String>
        where
            T: serde::ser::Serialize,
    {

        let res = client
            .post(&format!("http://localhost:8001/{}", path))
            .json(&body)
            .send();
        Some(res.unwrap().text().unwrap())
    }


}

