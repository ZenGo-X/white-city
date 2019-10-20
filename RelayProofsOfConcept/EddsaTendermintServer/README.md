# TendermintEdDSA

This is a POC for running distributed multi-party signatures with Tendermint consensus as backed for message broadcast

## Instructions: Single node 
You need to have tendermint installed.
Follow the installation guide for your system at [tendermint github](https://github.com/tendermint/tendermint)

0. Create an initial configuration file for a single node `tendermint init`

1. Build the repository with `cargo build --all`. This creates executables for both the server and client side

2. Reset tendermint state with `tendermint unsafe_reset_all`

3. Run Tendermint node: `tendermint node`

4. In a separate terminal window, run the application: `cargo run`

5. In yet another terminal window, run the key generation demo `./kg-demo.sh`

You can set the parameter of the number of clients runnig the protocol in `kg-demo.sh`

## Instructions: Tendermint cluster
./tools directory holds scripts to run the demo with a 4 node tendermint cluster.
Any one of the nodes can fail during the demo without compromising it.


1. run `./tools/local-cluster-init.sh` to create a 4 node testnet configuration
2. run `./tools/local-cluster-start.sh` to start 4 tendermint nodes, along with 4 running applications
3. run `./tools/kg-demo.sh` to create keys. By default, each client is communicating with a seperate node

At the moment, a reset needs to be performed after the key gen and before a signing example  
Reset the tendermint cluster with 
`./tools/local-cluster-reset.sh`

In the demo 5 clients create a threshold signature. A cluster of 4 nodes runs the protocol, after node 3 fails, the protocol still completes successfully.

To run the signing demo, the SMR state needs to be restarted.
Run `./tools/local-cluster-reset.sh`  
Then `./tools/sign-demo.sh`

![demo](./demo/tendermint-demo.gif)

