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
extern crate clap;
extern crate futures;
extern crate relay_server;
extern crate relay_server_common;
extern crate tokio_core;
extern crate tokio_io;

use clap::{App, Arg, ArgMatches};
use relay_server::start_server;
use std::net::SocketAddr;

fn arg_matches<'a>() -> ArgMatches<'a> {
    App::new("relay-server")
        .arg(
            Arg::with_name("address")
                .default_value("127.0.0.1:8080")
                .value_name("<HOST:PORT>"),
        )
        .arg(
            Arg::with_name("capacity")
                .default_value("2")
                .short("P")
                .long("participants"),
        )
        .get_matches()
}

fn main() {
    let matches = arg_matches();

    let addr: SocketAddr = matches
        .value_of("address")
        .unwrap()
        .parse()
        .expect("Unable to parse socket address");
    println!("{:?}", addr);

    let capacity: u32 = matches
        .value_of("capacity")
        .unwrap()
        .parse()
        .expect("Invalid number of participants");

    start_server(&addr, capacity);
}
