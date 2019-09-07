CWD=`dirname $0`

TM_PERSISTENT_PEERS=\
$(tendermint show_node_id --home $HOME/.tendermint/cluster4/1)"@127.0.0.1:46156,"\
$(tendermint show_node_id --home $HOME/.tendermint/cluster4/2)"@127.0.0.1:46256,"\
$(tendermint show_node_id --home $HOME/.tendermint/cluster4/3)"@127.0.0.1:46356,"\
$(tendermint show_node_id --home $HOME/.tendermint/cluster4/4)"@127.0.0.1:46456"

tmux new -d -s app1 && tmux send-keys -t app1 "cargo run -- --address 127.0.0.1:46158" C-m
tmux new -d -s app2 && tmux send-keys -t app2 "cargo run -- --address 127.0.0.1:46258" C-m
tmux new -d -s app3 && tmux send-keys -t app3 "cargo run -- --address 127.0.0.1:46358" C-m
tmux new -d -s app4 && tmux send-keys -t app4 "cargo run -- --address 127.0.0.1:46458" C-m

tmux new -d -s node1 && tmux send-keys -t node1 "tendermint unsafe_reset_all --home ~/.tendermint/cluster4/1 && tendermint node --proxy_app tcp://127.0.0.1:46158 --rpc.laddr=tcp://0.0.0.0:46157 --home ~/.tendermint/cluster4/1 --consensus.create_empty_blocks=false --p2p.laddr=tcp://0.0.0.0:46156 --p2p.persistent_peers=$TM_PERSISTENT_PEERS" C-m

tmux new -d -s node2 && tmux send-keys -t node2 "tendermint unsafe_reset_all --home ~/.tendermint/cluster4/2 && tendermint node --proxy_app tcp://127.0.0.1:46258 --rpc.laddr=tcp://0.0.0.0:46257 --home ~/.tendermint/cluster4/2 --consensus.create_empty_blocks=false --p2p.laddr=tcp://0.0.0.0:46256 --p2p.persistent_peers=$TM_PERSISTENT_PEERS" C-m

tmux new -d -s node3 && tmux send-keys -t node3 "tendermint unsafe_reset_all --home ~/.tendermint/cluster4/3 && tendermint node --proxy_app tcp://127.0.0.1:46358 --rpc.laddr=tcp://0.0.0.0:46357 --home ~/.tendermint/cluster4/3 --consensus.create_empty_blocks=false --p2p.laddr=tcp://0.0.0.0:46356 --p2p.persistent_peers=$TM_PERSISTENT_PEERS" C-m

tmux new -d -s node4 && tmux send-keys -t node4 "tendermint unsafe_reset_all --home ~/.tendermint/cluster4/4 && tendermint node --proxy_app tcp://127.0.0.1:46458 --rpc.laddr=tcp://0.0.0.0:46457 --home ~/.tendermint/cluster4/4 --consensus.create_empty_blocks=false --p2p.laddr=tcp://0.0.0.0:46456 --p2p.persistent_peers=$TM_PERSISTENT_PEERS" C-m
