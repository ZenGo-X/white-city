//! Implementation of a server designed to work
//! as a relay between Peers communicating in a MPC protocol.
//! A protocol is represented by a unique identifier and a capacity
//! A client that wishes to communicate with othr peers via the server
//! must build its messages in the Codec supplied in relay_server_common lib
//!
//!
//! To run the server: run this file and in another terminal, run:
//!     cargo +nightly run --example connect 127.0.0.1:8080
//! this will run a client that utilizes the server in some way
extern crate chrono;
extern crate futures;
extern crate relay_server;
extern crate relay_server_common;
extern crate structopt;
extern crate tokio_core;
extern crate tokio_io;

use relay_server::{
    resolve_client_msg_type, start_server, Client, ClientMessageType, RelaySession,
};
use std::net::SocketAddr;
use structopt::StructOpt;

// Argument parsing
#[derive(StructOpt, Debug)]
#[structopt(name = "relay-server")]
struct Opt {
    /// Number of participants in the protocol
    #[structopt(short = "P", long = "participants", default_value = "2")]
    capacity: u32,

    /// Address the server listens on
    #[structopt(name = "ADDRESS", default_value = "127.0.0.1:8080")]
    address: String,
}

fn main() {
    let opt = Opt::from_args();

    let addr = opt.address;
    let addr: SocketAddr = addr.parse().expect("Unable to parse socket address");
    println!("{:?}", addr);

    start_server(&addr, opt.capacity);
}
