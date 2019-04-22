///
/// structures for supported protocols for relay-server
///
use serde_json::{Result, Value};
use serde_json::Map;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::cell::RefCell;
use std::error::Error;

use ProtocolIdentifier;

static PROTOCOLS_F:&str = r#"./protocols.json"#;
use std::io::prelude::*;

//#[derive(Deserialize, Debug)]
//pub struct Protocols(serde_json::Map<String,Value>);

#[derive(Debug, Clone)]
pub struct ProtocolDescriptor {
    pub id: ProtocolIdentifier,
    pub capacity: u32,
    pub turn: RefCell<u32>,
}

impl ProtocolDescriptor {

    pub fn advance_turn(&self) -> u32 {
        let turn = self.turn.clone().into_inner();
        let peer_number = (turn + 1) % (self.capacity + 1);
        if peer_number == 0 {
            self.turn.replace(1);
        }else { self.turn.replace(peer_number); }
        return self.turn.clone().into_inner();
    }

    // get the # of next peer that can send a message
    pub fn next(&self) -> u32 {
        let turn = self.turn.clone().into_inner();
        turn
    }
}

impl ProtocolDescriptor{
        pub fn new(id: ProtocolIdentifier, capacity: u32) -> ProtocolDescriptor{
        ProtocolDescriptor{
            id,
            capacity,
            turn: RefCell::new(1),
        }
    }
}



pub fn is_valid_protocol(p:&ProtocolDescriptor) -> bool {
    let all_protocols = get_protocols();
    match all_protocols {
        Ok(_protocols) => {
            for prot in _protocols.protocols{
                println!("Checking if fits protocol: {:?}", prot);
                if prot.id == p.id {
                    if prot.capacities.contains(&(p.capacity)){
                        return true
                    }
                    else { return false }
                }
            }
            return false
//            match protocols.get(&p.id) {
//                Ok(prot) =>{
//                    let capacities = prot["capacities"];
//                    match  capacities.iter().find(|&&x| x == p.capacity){
//                        Ok(c) => return true,
//                        None => return false
//                    }
//                },
//                None => return false
//            }
        },
        Err(e) => panic!("corrupt protocols file")
    }
}

fn get_protocols() -> Result<Protocolss> {
        println!("Getting protocols");

        // Open the file in read-only mode with buffer.
        let path = PROTOCOLS_F;
        let mut file = File::open(path)?;
        let reader = BufReader::new(file);



        // Read the JSON contents of the file as an instance of `Protocols`.
        let p = serde_json::from_reader(reader)?;
        println!("Got protocols: {:?}", p);
        Ok(p)
}

#[derive(Deserialize, Debug)]
struct Protocolss {
    pub protocols: Vec<Protocol>,
}

#[derive(Deserialize, Debug)]
struct Protocol {
    pub id: u32,
    pub capacities: Vec<u32>,
    pub names: Vec<String>,
}