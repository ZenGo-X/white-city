use futures::sync::mpsc;
use relay_server::Client;
use relay_server::RelaySession;
use relay_server_common::ProtocolIdentifier;
use std::net::SocketAddr;

#[test]
fn test_server_add_peer() {
    let protocol_id: ProtocolIdentifier = 0;
    let capacity: u32 = 1;

    let client_addr: SocketAddr = "127.0.0.1:8081".parse().unwrap();

    start_server(&addr, capacity);

    //rs.insert_new_connection(client_addr.clone(), Client::new(tx));

    //let peer_num = rs.register_new_peer(client_addr, protocol_id, capacity);
    assert_eq!(1, 1);
}
