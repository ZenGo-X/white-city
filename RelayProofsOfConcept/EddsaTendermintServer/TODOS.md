# TODOs

+ tendermint docker
    * https://tendermint.com/docs/networks/docker-compose.html
    * white-city:
        - `--proxy_app tcp://127.0.0.1:46158 --rpc.laddr=tcp://0.0.0.0:46157 --p2p.laddr=tcp://0.0.0.0:46156 --p2p.persistent_peers=$TM_PERSISTENT_PEERS`
        - `I[2019-10-23|14:02:35.278] Started node                                 module=main nodeInfo="{ProtocolVersion:{P2P:7 Block:10 App:0} ID_:21a7e0f619adc7aa155f58a96e49074e74095ce9 ListenAddr:tcp://0.0.0.0:46156 Network:chain-tD4PZl Version:0.32.4 Channels:4020212223303800 Moniker:node0 Other:{TxIndex:on RPCAddress:tcp://0.0.0.0:46157}}"`
        - we care about 46158 & 46157 here
    * compose:
        - `node0    | I[2019-10-23|05:52:48.325] Started node                                 module=main nodeInfo="{ProtocolVersion:{P2P:7 Block:10 App:1} ID_:f26598257e5d767f94fa8c7db329275cbc2d6b0f ListenAddr:tcp://0.0.0.0:26656 Network:chain-7SreKK Version:0.32.4 Channels:4020212223303800 Moniker:FD25FB6CD8A5E049 Other:{TxIndex:on RPCAddress:tcp://0.0.0.0:26657}}"`
        - `node6    | I[2019-10-23|05:52:44.450] Started node                                 module=main nodeInfo="{ProtocolVersion:{P2P:7 Block:10 App:1} ID_:ee64807aa1ab432cf781c52db7d29fc01b901c18 ListenAddr:tcp://0.0.0.0:26656 Network:chain-7SreKK Version:0.32.4 Channels:4020212223303800 Moniker:A7753ED8581FB37B Other:{TxIndex:on RPCAddress:tcp://0.0.0.0:26657}}"`
    * what is "proxy_app"? 
    * all in 1 compose.yml?
    * port mapping?
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