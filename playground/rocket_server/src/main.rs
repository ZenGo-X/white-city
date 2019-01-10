#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

extern crate rocket_contrib;
extern crate reqwest;

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
pub struct Database {
   pub map: Mutex<HashMap<TupleKey, String>>,
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
    db_mtx: State<Mutex<HashMap<TupleKey,String>>>,
    request: Json<Index>
) -> Json<Result<Entry,()>> {
    let index: Index = request.0;
    let mut hm = db_mtx.lock().unwrap();
    match hm.get(&index.key){
        Some(v) => {
            let entry = Entry{
                key: index.key,
                value: format!("{} {}", v.clone(), "please".to_string()),
            };
            Json(Ok(entry))
        },
        None => Json(Err(())),
    }

}

#[post("/set", format = "json", data = "<request>")]
fn set(
    db_mtx: State<Mutex<HashMap<TupleKey,String>>>,
    request: Json<Entry>
) -> Json<Result<(),()>> {
    let entry: Entry = request.0;
    let mut hm = db_mtx.lock().unwrap();
    hm.insert(entry.key.clone(), entry.value.clone());
    Json(Ok(()))
}
//refcell, arc

fn main() {
    let key1 = TupleKey::new("1".to_string(), "2".to_string());
   // let mut db = Database{ map: HashMap::new()};
    let db : HashMap<TupleKey, String> = HashMap::new();
    let db_mtx = Mutex::new(db);
  //  db.insert(key1, "test".to_string());



    rocket::ignite().mount("/", routes![get, set]).manage(db_mtx).launch();
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
            .post(&format!("http://127.0.0.1:8000/{}", path))
            .json(&body)
            .send();
        Some(res.unwrap().text().unwrap())
    }


}

