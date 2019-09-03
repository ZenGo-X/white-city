**To Run 2p-EdDSA** 

This is a POC for running distributed multi-party signatures with Tendermint consensus as backed for message broadcast

## Instructions
You need to have tendermint installed.
Follow the installation guide for your system at [tendermint github](https://github.com/tendermint/tendermint)

1. Run Tendermint node: `tendermint node` (To reset the state between runs, execute `tendermint unsafe_reset_all`)

2. In a separate terminal window, run the application: `cargo run`

3. In yet another terminal window, run the key generation client `cargo run --example keygen-client -- -P 1`

TODO:
Add explanation on deploying a cluster + Dockerfile to automate the process
