# white-city
Network layer for sMPC (Secure Multi-Party Computation) protocols 
### Goal: 
Build an API for a p2p network that will let the protocol implementor easy network integration for his MPC protocol. 
### Project Status: 
The project is moving on multiple fronts
- **Multi-party-eddsa:** a server-client web framework based on [Rocket](https://rocket.rs/) with a client running [multi party eddsa](https://github.com/KZen-networks/multi-party-eddsa/wiki/Aggregated-Ed25519-Signatures) as an example. code:  [pg-eddsa-client](https://github.com/KZen-networks/white-city/tree/master/playground/pg-eddsa-client) , [rocket-server](https://github.com/KZen-networks/white-city/tree/master/playground/rocket_server). This is a playground proejct for fast testing various concepts. 
- **Server-relay:** a low level server-client framework based on [Tokio](https://tokio.rs/). This framework will enable a transition to a full p2p network. Currently WIP, there is a chat-like [application](https://github.com/KZen-networks/white-city/blob/master/RelayServer/relay-server/src/main.rs) to transfer messages between peers in a round robin based fasion. 
- **Formal-spec:** This is an attempt to capture MPC protocols network layer as a distributed system. Currently there is a [TLA+ spec](https://github.com/KZen-networks/white-city/tree/master/RelayServer/TLA%2B) and equivalent [Coq spec](https://github.com/KZen-networks/white-city/tree/master/RelayServer/coq) of the Server-relay 
### Reference implementations: 
1. **Bar Ilan::ACP** - https://github.com/cryptobiu/ACP. Network is based on proxy.
2. **EPFL::DEDIS** - https://dedis.epfl.ch/ and specifcally https://github.com/dedis/onet
3. **KULeuven-COSIC::Scale-Mamba** - https://github.com/KULeuven-COSIC/SCALE-MAMBA
4. **aicis::fresco** - https://github.com/aicis/fresco
