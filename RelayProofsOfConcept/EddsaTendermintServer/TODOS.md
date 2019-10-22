# TODOs

+ tendermint docker
    * https://tendermint.com/docs/networks/docker-compose.html
    * `--proxy_app tcp://127.0.0.1:46158 --rpc.laddr=tcp://0.0.0.0:46157 --p2p.laddr=tcp://0.0.0.0:46156 --p2p.persistent_peers=$TM_PERSISTENT_PEERS`
    * dialing timeouts?
        - https://github.com/tendermint/tendermint/issues/3178
        - https://github.com/tendermint/tendermint/issues/1408
        - https://github.com/tendermint/tendermint/issues/1427
        - https://github.com/tendermint/tendermint/issues/2815
        - https://forum.cosmos.network/t/error-dialing-peers-running-full-node/576
        - https://forum.cosmos.network/t/problem-connecting-validator-nodes-error-failed-to-decrypt-secretconnection/829
+ server docker
+ kg docker
+ sign-client docker