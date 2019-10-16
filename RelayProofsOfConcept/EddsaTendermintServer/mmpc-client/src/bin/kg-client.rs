use std::net::SocketAddr;
use std::{thread, time};

use clap::{App, Arg, ArgMatches};
use log::debug;

use mmpc_client::eddsa_peer::EddsaPeer;
use mmpc_client::tendermint_client::SessionClient;

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
                .short("P")
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
        .get_matches()
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

    let index: u32 = matches
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

    let port = 8080 + index;
    let proxy_addr = format!("tcp://{}", proxy);
    let client_addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let mut session: SessionClient<EddsaPeer> =
        SessionClient::new(client_addr, &proxy_addr.parse().unwrap(), capacity);
    let server_response = session.register(index, capacity);
    let next_message = session.generate_client_answer(server_response);
    debug!("Next message: {:?}", next_message);
    // TODO The client/server response could be an error
    let server_response = session.send_message(next_message.unwrap());
    session.store_server_response(&server_response);

    debug!("Server Response: {:?}", server_response);

    loop {
        let round = session.state.data_manager.data_holder.current_step;
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
        thread::sleep(time::Duration::from_millis(100));
    }
}
