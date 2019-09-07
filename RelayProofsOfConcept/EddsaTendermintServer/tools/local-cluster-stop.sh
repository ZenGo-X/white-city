tmux kill-session -t app1
tmux kill-session -t app2
tmux kill-session -t app3
tmux kill-session -t app4


tmux kill-session -t node1
tmux kill-session -t node2
tmux kill-session -t node3
tmux kill-session -t node4

pgrep tendermint | xargs kill -KILL
