use std::io;
use std::net::SocketAddr;
use std::{thread, time};

use clap::{App, Arg, ArgMatches};
use log::debug;

use mmpc_client::eddsa_peer_kg::EddsaPeer;
use mmpc_client::peer::Peer;
use mmpc_client::tendermint_client::SessionClient;

const MAX_RETRY: u32 = 30;
const RETRY_TIMEOUT: u64 = 100;

fn arg_matches<'a>() -> ArgMatches<'a> {
    App::new("relay-server")
        .arg(
            Arg::with_name("index")
                .short("I")
                .long("index")
                .default_value("1"),
        )
        .arg(
            Arg::with_name("capacity")
                .default_value("2")
                .short("C")
                .long("capacity"),
        )
        .arg(
            Arg::with_name("filename")
                .default_value("keys")
                .long("filename")
                .short("F"),
        )
        .arg(
            Arg::with_name("proxy")
                .default_value("127.0.0.1:26657")
                .long("proxy"),
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

fn setup_logging(verbosity: u64, index: u32) -> Result<(), fern::InitError> {
    let mut base_config = fern::Dispatch::new();

    base_config = match verbosity {
        0 => base_config
            .level(log::LevelFilter::Info)
            .level_for("abci::server", log::LevelFilter::Warn), // filter out abci::server
        1 => base_config
            .level(log::LevelFilter::Debug)
            .level_for("tokio_core", log::LevelFilter::Warn) // filter out tokio
            .level_for("tokio_reactor", log::LevelFilter::Warn)
            .level_for("hyper", log::LevelFilter::Warn),
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
        .chain(fern::log_file(format!("log-kg-{}.log", index))?);

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

pub enum MessageProcessResult {
    Message,
    NoMessage,
    Abort,
}

fn main() {
    better_panic::Settings::debug()
        .most_recent_first(false)
        .lineno_suffix(true)
        .install();

    let matches = arg_matches();

    let client_index: u32 = matches
        .value_of("index")
        .unwrap()
        .parse()
        .expect("Unable to parse index");

    let capacity: u32 = matches
        .value_of("capacity")
        .unwrap()
        .parse()
        .expect("Invalid number of participants");

    let proxy: String = matches
        .value_of("proxy")
        .unwrap()
        .parse()
        .expect("Invalid proxy address");

    let verbosity: u64 = matches.occurrences_of("verbose");

    setup_logging(verbosity, client_index).expect("failed to initialize logging.");

    let port = 8080 + client_index;
    let proxy_addr = format!("tcp://{}", proxy);
    let client_addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    debug!("Capacity flag is {}", capacity);
    let mut session: SessionClient<EddsaPeer> = SessionClient::new(
        client_addr,
        &proxy_addr.parse().unwrap(),
        client_index,
        capacity,
        Vec::new(),
    );
    let server_response = session.register(client_index, capacity);
    let next_message = session.generate_client_answer(server_response);
    debug!("Next message: {:?}", next_message);
    // TODO The client/server
    let server_response = session.send_message(next_message.unwrap());
    session.store_server_response(&server_response);

    debug!("Server Response: {:?}", server_response);

    for _ in { 1..MAX_RETRY } {
        let round = session.state.data_manager.data_holder.current_step();
        debug!("Now on round {}", round);
        if session.state.stored_messages.get_number_messages(round) == capacity as usize {
            for msg in session
                .state
                .stored_messages
                .get_messages_vector_client_message(round)
            {
                session.handle_relay_message(msg.clone());
            }
            return;
        }
        let server_response = session.query();
        session.store_server_response(&server_response);
        thread::sleep(time::Duration::from_millis(RETRY_TIMEOUT));
    }
}
