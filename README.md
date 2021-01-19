# white-city
API to integrate distributed network for secure computation protocols. 

Read more details in our technical report:  
[White-City](./White-City-Report/whitecity_new.pdf)

### Background
Secure Multiparty Computation (MPC) has transitioned from a thoretical field to applied technology with real life use cases. In MPC a set of n parties are running a distributed computation over private inputs. To do so, MPC protocols designers make assumptions on the required network and communication channels. A complete p2p network setup might turn out to be costly, effectively eliminating the practicallity of running MPC at scale. 

Instead, we suggest using untrusted coordinator, connected in a star topology to all clients. This gets us immidiate improvment on communication complexity of simple p2p, and potentially benefits robustness, accountabillity and fault tolarance. 


### Project Status: 
The current stage is focused on the idea of replicated state machine. The repo contains three proofs of concepts. 
The latest implementation uses Tendermint to replicate the state machine across a set of known servers.
Clients broadcast transactions to the servers to change the state, and read messages from the public bulletin board. Older PoCs are using a single untrusted coordinator. 

- **[Tendermint](https://github.com/KZen-networks/white-city/tree/master/RelayProofsOfConcept/EddsaTendermintServer):** Broadcast channel using Tendermint as an immutable bulletin board.
- **[TokioServer](https://github.com/KZen-networks/white-city/tree/master/RelayProofsOfConcept/EddsaTokioServer):** a socket level implementation using Tokio Crate.
- **[RocketServer](https://github.com/KZen-networks/white-city/tree/master/RelayProofsOfConcept/EddsaRocketServer):** a Http server implementation using Rocket crate. 
Proofs of concept are currently running [multi party EdDSA](https://github.com/KZen-networks/multi-party-eddsa) library. In general, all messages in the MPC protocol should be broadcast messages (p2p messages are broadcasted encrypted). 

As a side project there is also an effort to formally verify the centralized state machine model in [Coq/TLA+](https://github.com/KZen-networks/white-city/tree/master/RelayProofsOfConcept/Formal-spec)

### Hall of Fame: 
Here is a list of contributors to White City (not ordered): 
- Avi Kozokin
- Alex Manuskin
- Frederic Peschanski
- Omer Shlomovits 
- Roman Zeyde
- Haoyu LIN


### Want to Contribute:
Please send an email to github@kzencorp.com containing your github username. We will get in touch and bring you up to speed. We try to keep the list of issues relevant so it might also be a good place to start. Join the ZenGo X [Telegram](https://t.me/joinchat/ET1mddGXRoyCxZ-7) for discussions on code and research.
