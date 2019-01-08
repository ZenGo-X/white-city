extern crate tokio;
#[macro_use]
extern crate serde_json;
extern crate tokio_serde_json;
extern crate futures;

use std::str;
use std::collections::HashMap;
use std::io::BufReader;
use std::env;
use std::iter;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::io::lines;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio::codec::{FramedRead, LengthDelimitedCodec, FramedWrite};

use serde_json::value::Value;
use tokio_serde_json::{ReadJson, WriteJson};
use std::fmt;
use tokio::io;
//use future::Future;
use futures::stream::{self, Stream};
use std::io::{Error, ErrorKind};

#[derive(Hash)]
#[derive(PartialEq, Eq, Clone)]
struct TupleKey {
    first: String,
    second: String
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

/// The in-memory database shared amongst all clients.
///
/// This database will be shared via `Arc`, so to mutate the internal map we're
/// going to use a `Mutex` for interior mutability.
struct _Database {
    map: Mutex<HashMap<TupleKey, String>>,
}

/// Possible requests our clients can send us
enum _Request {
    Get { key: TupleKey },
    Set { key: TupleKey, value: String },
    Error { },

}

/// Responses to the `Request` commands above
enum _Response {
    Value { key: TupleKey, value: String },
    Set { key: TupleKey, value: String, previous: Option<String> },
    Error { msg: String },
}

struct Database {
    map: Mutex<HashMap<String, String>>,
}

/// Possible requests our clients can send us
enum Request {
    Get { key: String },
    Set { key: String, value: String },
}

/// Responses to the `Request` commands above
enum Response {
    Value { key: String, value: String },
    Set { key: String, value: String, previous: Option<String> },
    Error { msg: String },
}

fn main() -> Result<(), Box<std::error::Error>> {
    // Parse the address we're going to run this server on
    // and set up our TCP listener to accept connections.
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());
    let addr = addr.parse::<SocketAddr>()?;
    let listener = TcpListener::bind(&addr).map_err(|_| "failed to bind")?;
    println!("Listening on: {}", addr);

    // Create the shared state of this server that will be shared amongst all
    // clients. We populate the initial database and then create the `Database`
    // structure. Note the usage of `Arc` here which will be used to ensure that
    // each independently spawned client will have a reference to the in-memory
    // database.
    let mut initial_db = HashMap::new();
    initial_db.insert("foo".to_string(), "bar".to_string());
    let db = Arc::new(Database {
        map: Mutex::new(initial_db),
    });

    let done = listener.incoming()
        .map_err(|e| println!("error accepting socket; error = {:?}", e))
        .for_each(move |socket| {
            // As with many other small examples, the first thing we'll do is
            // *split* this TCP stream into two separately owned halves. This'll
            // allow us to work with the read and write halves independently.
            let (reader, writer) = socket.split();

                        let lines2 = lines(BufReader::new(reader))
                            .map( |line| {
                                format!("{} is {}", line.clone(), line.clone())

                           });


                        let writes
                        = lines2.fold(writer, |writer, mut response| {
                            response.push('\n');
                            io::write_all(writer, response.into_bytes()).map(|(w, _)| w)
                        });



            let msg
            = writes.then(move |_| Ok(()));

            tokio::spawn(msg)

        });




    tokio::run(done);
    Ok(())
}


/*
impl Request {

    fn parse(input: &Value) -> Result<Request, String> {
        let party = input["party"].clone();
        let message_type = input["message_type"].clone();
        let tk = TupleKey::new(party.to_string(), message_type.to_string());
        let get = Value::String("GET".to_string());
        let set = Value::String("SET".to_string());

        match input["op"].clone() {
            get => {
                Ok(Request::Get { key: tk })
            }
            set => {
                let value = input["message"].clone();
                Ok(Request::Set { key: tk, value: value.to_string() })

            }
        //    Some(cmd) => Err(format!("unknown command: {}", cmd)),
        //    None => Err(format!("empty input")),
        }
    }
}

impl Response {
    fn serialize(&self) -> String {
        match *self {
            Response::Value {  ref key, ref value } => {
                let jsn = json!({
                        "op": "GET",
                        "party": key.first,
                        "message_type": key.second,
                        "message": value,
                    });
                serde_json::to_string(&jsn).unwrap()
            }
            Response::Set { ref key, ref value, ref previous } => {
                let jsn = json!({
                        "op": "SET",
                        "party": key.first,
                        "message_type": key.second,
                        "message": value,
                    });
                serde_json::to_string(&jsn).unwrap()
            }
            Response::Error { ref msg } => {
                format!("error: {}", msg)
            }
        }
    }
}

*/