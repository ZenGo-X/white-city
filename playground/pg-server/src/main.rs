extern crate tokio;
#[macro_use]
extern crate serde_json;
extern crate tokio_serde_json;

use std::collections::HashMap;
use std::io::BufReader;
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::io::{lines, write_all};
use tokio::net::TcpListener;
use tokio::prelude::*;
use tokio::codec::{FramedRead, LengthDelimitedCodec, FramedWrite};

use serde_json::value::Value;
use tokio_serde_json::{ReadJson, WriteJson};
use std::fmt;


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
struct Database {
    map: Mutex<HashMap<TupleKey, String>>,
}

/// Possible requests our clients can send us
enum Request {
    Get { key: TupleKey },
    Set { key: TupleKey, value: String },
    Error { },

}

/// Responses to the `Request` commands above
enum Response {
    Value { key: TupleKey, value: String },
    Set { key: TupleKey, value: String, previous: Option<String> },
    Error { msg: String },
}

fn main()  {
    // Parse the address we're going to run this server on
    // and set up our TCP listener to accept connections.
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:17653".to_string());
    let addr = addr.parse::<SocketAddr>().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();
    println!("Listening on: {}", addr);

    // Create the shared state of this server that will be shared amongst all
    // clients. We populate the initial database and then create the `Database`
    // structure. Note the usage of `Arc` here which will be used to ensure that
    // each independently spawned client will have a reference to the in-memory
    // database.
    let mut initial_db = HashMap::new();
    //initial_db.insert("foo".to_string(), "bar".to_string());
    let db = Arc::new(Database {
        map: Mutex::new(initial_db),
    });

    tokio::run(
        listener
            .incoming()
            .map_err(|e| println!("error accepting socket; error = {:?}", e))
            .for_each( |socket| {
                let (reader, writer) = socket.split();

                // Delimit frames using a length header
                let length_delimited = FramedRead::new(reader, LengthDelimitedCodec::new());

                // Deserialize frames
                let deserialized = ReadJson::<_, Value>::new(length_delimited)
                    .map_err(|e| println!("ERR: {:?}", e));

                let length_delimited_write = FramedWrite::new(writer, LengthDelimitedCodec::new());
                let serialized = WriteJson::new(length_delimited_write);
                // Spawn a task that prints all received messages to STDOUT
                tokio::spawn( deserialized.for_each(|msg| {
                    serialized.send(msg).map(|_| ());
                  //  println!("GOT: {:?}", msg);
                    Ok(())
                }));

                Ok(())
            }).map_err(|_| ()),
    );

    /*
    tokio::run(
        listener
            .incoming()
            .for_each(|socket| {
                let (reader, writer) = socket.split();
                // Delimit frames using a length header
                let length_delimited_read = FramedRead::new(reader, LengthDelimitedCodec::new());
                let length_delimited_write = FramedWrite::new(writer, LengthDelimitedCodec::new());

                // Deserialize frames
                let deserialized = ReadJson::<_, Value>::new(length_delimited_read)
                    .map_err(|e| println!("ERR: {:?}", e));
                let serialized = WriteJson::new(length_delimited_write);
                // Spawn a task that prints all received messages to STDOUT
                tokio::spawn(deserialized.for_each(|msg| {
                    serialized.send(msg).map(|_| ())
                }));

                Ok(())
            }).map_err(|_| ()),
    );

    */

    /*
    tokio::run(
        socket
            .and_then(|socket| {
                // Delimit frames using a length header
                let length_delimited = FramedWrite::new(socket, LengthDelimitedCodec::new());

                // Serialize frames with JSON
                let serialized = WriteJson::new(length_delimited);

                // Send the value
                serialized
                    .send(json!({
                        "name": "John Doe",
                        "age": 43,
                        "phones": [
                            "+44 1234567",
                            "+44 2345678"
                        ]
                    })).map(|_| ())
            }).map_err(|_| ()),
    );
    tokio::run( listener.incoming()
        .map_err(|e| println!("error accepting socket; error = {:?}", e))
        .for_each(move |socket| {
            // As with many other small examples, the first thing we'll do is
            // *split* this TCP stream into two separately owned halves. This'll
            // allow us to work with the read and write halves independently.
         //   let (reader, writer) = socket.split();

            // Since our protocol is line-based we use `tokio_io`'s `lines` utility
            // to convert our stream of bytes, `reader`, into a `Stream` of lines.

            let length_delimited = FramedRead::new(socket, LengthDelimitedCodec::new());

            let deserialized = ReadJson::<_, Value>::new(length_delimited).map_err(|e| println!("ERR: {:?}", e));

            tokio::spawn(deserialized.for_each(|val| {
                let request = match Request::parse(&val) {
                    Ok(req) => req,
                    Err(e) => Request::Error { },
                };
                let mut db = db.map.lock().unwrap();

                let server_response =  match request {
                    Request::Get { key } => {
                        match db.get(&key) {
                            Some(value) => Response::Value { key, value: value.clone() },
                            None => Response::Error { msg: format!("no key {:?}", key.first) },
                        }
                    }
                    Request::Set { key, value } => {
                        let previous = db.insert(key.clone(), value.clone());
                        Response::Set { key, value, previous }
                    }
                };
                let length_delimited_write = FramedWrite::new(socket, LengthDelimitedCodec::new());
                let serialized = WriteJson::new(length_delimited_write);
                 serialized.send(server_response.serialize()).map(|_| ()).map(|_| ());

                Ok(())
            }));


        }).map_err(|_| ()),
    );
    Ok(())
    */
}

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

