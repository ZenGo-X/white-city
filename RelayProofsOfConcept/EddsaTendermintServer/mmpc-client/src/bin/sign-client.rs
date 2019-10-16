/// Implementation of a client that communicates with the relay server
/// This client represents eddsa peer
use std::net::SocketAddr;
use std::{thread, time};

use clap::{App, Arg, ArgMatches};
use log::debug;

use mmpc_client::eddsa_peer_sign::EddsaPeer;
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
            Arg::with_name("message")
                .default_value("message")
                .long("message")
                .short("M"),
        )
        .arg(
            Arg::with_name("proxy")
                .default_value("127.0.0.1:26657")
                .long("proxy"),
        )
        .get_matches()
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

    let message: String = matches
        .value_of("message")
        .unwrap()
        .parse()
        .expect("Invalid message to sign");

    let proxy: String = matches
        .value_of("proxy")
        .unwrap()
        .parse()
        .expect("Invalid proxy address");

    let message_to_sign = match hex::decode(message.to_owned()) {
        Ok(x) => x,
        Err(_) => message.as_bytes().to_vec(),
    };

    // Port and ip address are used as a unique indetifier to the server
    // This should be replaced with PKi down the road
    let port = 8080 + index;
    let proxy_addr = format!("tcp://{}", proxy);
    let client_addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let mut session: SessionClient<EddsaPeer> = SessionClient::new(
        client_addr,
        // TODO: pass tendermint node address as parameter
        &proxy_addr.parse().unwrap(),
        index,
        capacity,
        message_to_sign,
    );
    let server_response = session.register(index, capacity);
    let mut next_message = session.generate_client_answer(server_response);
    println!("Next message: {:?}", next_message);
    // TODO The client/server response could be an error
    let mut server_response = session.send_message(next_message.clone().unwrap());
    session.store_server_response(&server_response);
    // Number of rounds in signing
    let rounds = 4;
    'outer: for _ in 0..rounds {
        'inner: loop {
            let round = session.state.data_manager.data_holder.current_step;
            if session.state.stored_messages.get_number_messages(round) == capacity as usize {
                for msg in session
                    .state
                    .stored_messages
                    .get_messages_vector_client_message(round)
                {
                    next_message = session.handle_relay_message(msg.clone());
                }
                // Do not send response on last round
                if round != rounds - 1 {
                    server_response = session.send_message(next_message.clone().unwrap());
                    session.store_server_response(&server_response);
                }
                break 'inner;
            } else {
                let server_response = session.query();
                // println!("Server response {:?}", server_response);
                // println!("Server response len {}", server_response.keys().len());
                session.store_server_response(&server_response);
                thread::sleep(time::Duration::from_millis(100));
                // println!("All stored messages {:?}", session.state.stored_messages);
            }
        }
    }
}
