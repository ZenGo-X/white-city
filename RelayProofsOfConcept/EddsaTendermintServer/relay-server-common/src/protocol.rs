/// Structures for supported protocols for relay-server
use log::debug;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::sync::{Arc, RwLock};

use crate::ProtocolIdentifier;

static PROTOCOLS_F: &str = r#"./protocols.json"#;

#[derive(Debug, Clone)]
pub struct ProtocolDescriptor {
    pub id: ProtocolIdentifier,
    pub capacity: u32,
    pub turn: Arc<RwLock<u32>>,
}

impl ProtocolDescriptor {
    pub fn new(id: ProtocolIdentifier, capacity: u32) -> ProtocolDescriptor {
        ProtocolDescriptor {
            id,
            capacity,
            turn: Arc::new(RwLock::new(1)),
        }
    }

    // Advances the peer whose turn it is to transmit.
    // If the peer is 0, initializes state to 1, else, advances turn by 1
    pub fn advance_turn(&self) -> u32 {
        let mut turn = self.turn.write().unwrap();
        let peer_number = (*turn + 1) % (self.capacity + 1);
        if peer_number == 0 {
            *turn = 1;
        } else {
            *turn = peer_number;
        }
        *turn
    }

    // Get the # of next peer that can send a message
    pub fn next(&self) -> u32 {
        *self.turn.read().unwrap()
    }
}

/// Returns true if the protocol is a valid protocol as determined by the
/// protocols.json file
pub fn is_valid_protocol(p: &ProtocolDescriptor) -> bool {
    let all_protocols = get_protocols();
    match all_protocols {
        Ok(_protocols) => {
            for prot in _protocols.protocols {
                debug!("Checking if fits protocol: {:?}", prot);
                if prot.id == p.id {
                    if prot.capacities.contains(&(p.capacity)) {
                        return true;
                    } else {
                        return false;
                    }
                }
            }
            return false;
        }
        Err(_) => panic!("Corrupt protocols file"),
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Protocolss {
    pub protocols: Vec<Protocol>,
}

#[derive(Deserialize, Debug, Serialize)]
struct Protocol {
    pub id: u32,
    pub capacities: Vec<u32>,
    pub names: Vec<String>,
}

// Reutrn all avaliable protocols
fn get_protocols() -> Result<Protocolss, Box<dyn Error>> {
    debug!("Getting protocols");

    // Open the file in read-only mode with buffer.
    let path = PROTOCOLS_F;
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `Protocols`.
    let p = serde_json::from_reader(reader)?;

    //debug!("Got protocols: {:?}", p);
    Ok(p)
}
