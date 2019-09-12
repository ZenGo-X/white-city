# white-city
API to integrate distributed network for secure computation protocols. 

### Background
MPC communication models require use of concepts such as Termination, Rounds, Broadcast channel, p2p channel and so on. The need for this API came to us at KZen since we work with multiple MPC algorithms and we couldn't find a robust and easy to plug into distributed network layer that answer the specific needs of MPC protocols. We aimed to create a unified API that enjoys best practices and tools of distributed network technologies such as consensus, fault tolerance and more. We believe this framework can be of use to other MPC implementers and therefore we are building it in modular way to answer all types of MPC use cases. 

### Project Status: 
The current stage is focused on the idea of replicated state machine. The repo contain tree proof of concepts for centralized state machine where an untrusted coordinator (or coordinators) is maintaining a state of the protocol and parties are clients, reading and writing to the state. This gives a few benefits over message passing system: Clients can have down time and the protocol can run more offline.
The latest implementation uses Tendermint to replicate the state machine across a set of known servers.
Clients broadcast transactions to the servers to change the state, and read messages from the public bulletin board.

- **[Tendermint](https://github.com/KZen-networks/white-city/tree/master/RelayProofsOfConcept/EddsaTendermintServer):** Broadcast channel using Tendermint as an immutable bulletin board.
- **[TokioServer](https://github.com/KZen-networks/white-city/tree/master/RelayProofsOfConcept/EddsaTokioServer):** a socket level implementation using Tokio Crate.
- **[RocketServer](https://github.com/KZen-networks/white-city/tree/master/RelayProofsOfConcept/EddsaRocketServer):** a Http server implementation using Rocket crate. 
Proofs of concept are currently running [multi party EdDSA](https://github.com/KZen-networks/multi-party-eddsa) library. 

As a side project there is also an effort to formally verify the centralized state machine model in [Coq/TLA+](https://github.com/KZen-networks/white-city/tree/master/RelayProofsOfConcept/Formal-spec)

### Hall of Fame: 
Here is a list of contributors to White City (not ordered): 
- Avi Kozokin
- Alex Manuskin
- Frederic Peschanski
- Omer Shlomovits 
- Roman Zeyde


### Want to Contribute:
Please send an email to github@kzencorp.com containing your github username. We will get in touch and bring you up to speed. We try to keep the list of issues relevant so it might also be a good place to start. Join the KZen Research [Telegram]( https://t.me/kzen_research) for discussions on code and research.
