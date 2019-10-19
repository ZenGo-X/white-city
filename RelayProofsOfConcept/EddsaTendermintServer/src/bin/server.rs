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
//!
use clap::{App, Arg, ArgMatches};
use mmpc_server::RelayApp;
use std::io;
use std::net::SocketAddr;

fn arg_matches<'a>() -> ArgMatches<'a> {
    App::new("relay-server")
        .arg(
            Arg::with_name("address")
                // Default tendermint port
                .long("address")
                .short("A")
                .default_value("127.0.0.1:26658")
                .value_name("<HOST:PORT>"),
        )
        .arg(
            Arg::with_name("capacity")
                .default_value("2")
                .short("P")
                .long("participants"),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .multiple(true)
                .help("Increases logging verbosity each use for up to 3 times"),
        )
        .get_matches()
}

fn setup_logging(verbosity: u64, port: String) -> Result<(), fern::InitError> {
    let mut base_config = fern::Dispatch::new();

    base_config = match verbosity {
        0 => base_config
            .level(log::LevelFilter::Info)
            .level_for("abci::server", log::LevelFilter::Warn), // filter out abci::server
        1 => base_config
            .level(log::LevelFilter::Debug)
            .level_for("tokio_core", log::LevelFilter::Warn) // filter out tokio
            .level_for("tokio_reactor", log::LevelFilter::Warn)
            .level_for("hyper", log::LevelFilter::Warn)
            .level_for("abci::server", log::LevelFilter::Warn),
        _2_or_more => base_config.level(log::LevelFilter::Trace),
    };

    // Separate file config so we can include year, month and day in file logs
    let file_config = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {} {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                line!(),
                message
            ))
        })
        .chain(fern::log_file(format!("relay-server-{}.log", port))?);

    let stdout_config = fern::Dispatch::new()
        .format(|out, message, record| {
            // special format for debug messages coming from our own crate.
            if record.level() > log::LevelFilter::Info && record.target() == "relay_server" {
                out.finish(format_args!(
                    "---\nDEBUG: {}: {}\n---",
                    chrono::Local::now().format("%H:%M:%S"),
                    message
                ))
            } else {
                out.finish(format_args!(
                    "[{}][{}][{}] {} ",
                    chrono::Local::now().format("%H:%M:%S"),
                    record.target(),
                    record.level(),
                    message
                ))
            }
        })
        .chain(io::stdout());

    base_config
        .chain(file_config)
        .chain(stdout_config)
        .apply()?;

    Ok(())
}

fn main() {
    let matches = arg_matches();

    let addr: SocketAddr = matches
        .value_of("address")
        .unwrap()
        .parse()
        .expect("Unable to parse socket address");

    let capacity: u32 = matches
        .value_of("capacity")
        .unwrap()
        .parse()
        .expect("Invalid number of participants");

    let port = addr.port().to_string();

    let verbosity: u64 = matches.occurrences_of("verbose");

    setup_logging(verbosity, port).expect("failed to initialize logging.");

    abci::run(addr, RelayApp::new(capacity));
}
