import argparse
import os
import stat

HELP_MESSAGE = """
Config generator for experiments
"""

def create_init_file(nodes):
    init_file_name = './tools/local-cluster-init-{}.sh'.format(nodes)
    with open(init_file_name,
              'w') as init_file:
        init_file.write('tendermint testnet --v {0} --o ~/.tendermint/cluster{0}/'.format(nodes))
    st = os.stat(init_file_name)
    os.chmod(init_file_name, st.st_mode | stat.S_IEXEC)


def create_start_file(nodes):
    def persisnent_peers(nodes):
        config_lines = list()
        p2p_port_start = 46056
        for node in range(nodes - 1):
            p2p_port = p2p_port_start + node * 100
            node_config = '$(tendermint show_node_id --home $HOME/.tendermint/cluster{0}/node0)"@127.0.0.1:{1},"\\'.format(nodes,p2p_port)
            config_lines.append(node_config + '\n')
        # Last line without \
        p2p_port = p2p_port_start + (nodes - 1) * 100
        node_config = '$(tendermint show_node_id --home $HOME/.tendermint/cluster{0}/node0)"@127.0.0.1:{1}"'.format(nodes, p2p_port)
        config_lines.append(node_config + '\n')
        return config_lines

    def app_tmux_sessions(nodes):
        node_lines = list()
        proxy_address_base = 46058
        for node in range(nodes):
            proxy_port = proxy_address_base + node * 100
            line = 'tmux new -d -s app{0} && tmux send-keys -t app{0} "cargo run -- --address 127.0.0.1:{1}" C-m'.format(node, proxy_port) + '\n'
            node_lines.append(line)
        return node_lines

    def node_tmux_sessions(nodes):
        node_lines = list()
        proxy_address_base = 46058
        rpc_address_base = 46057
        p2p_address_base = 46056
        for node in range(nodes):
            proxy_port = proxy_address_base + node * 100
            rpc_port = rpc_address_base + node * 100
            p2p_port = p2p_address_base + node * 100
            line = 'tmux new -d -s node{0} && tmux send-keys -t node{0} "tendermint node --proxy_app tcp://127.0.0.1:{1} --rpc.laddr=tcp://0.0.0.0:{2} --home ~/.tendermint/cluster{3}/node{0} --consensus.create_empty_blocks=false --p2p.laddr=tcp://0.0.0.0:{4} --p2p.persistent_peers=$TM_PERSISTENT_PEERS" C-m'.format(node, proxy_port, rpc_port, nodes, p2p_port) + '\n'
            node_lines.append(line)
        return node_lines

    start_file_name = './tools/local-cluster-start-{}.sh'.format(nodes)
    with open(start_file_name,
              'w') as start_file:
        start_file.write('CWD=`dirname $0`' + '\n')
        start_file.write('TM_PERSISTENT_PEERS=\\' + '\n')
        start_file.writelines(persisnent_peers(nodes))
        start_file.writelines(app_tmux_sessions(nodes))
        start_file.writelines(node_tmux_sessions(nodes))
    st = os.stat(start_file_name)
    os.chmod(start_file_name, st.st_mode | stat.S_IEXEC)


def create_stop_file(nodes):
    stop_file_name = './tools/local-cluster-stop-{}.sh'.format(nodes)
    with open(stop_file_name,
              'w') as stop_file:
        for node in range(nodes):
            stop_file.write('tmux kill-session -t app{}'.format(node) + '\n')
            stop_file.write('tmux kill-session -t node{}'.format(node) + '\n')
        stop_file.write('pgrep tendermint | xargs kill -KILL')
    st = os.stat(stop_file_name)
    os.chmod(stop_file_name, st.st_mode | stat.S_IEXEC)

def create_delete_file(nodes):
    delete_file_name = './tools/local-cluster-delete-{}.sh'.format(nodes)
    with open(delete_file_name,
              'w') as delete_file:
        delete_file.write('CWD=`dirname $0`' + '\n')
        delete_file.write('$CWD/local-cluster-stop-{}.sh'.format(nodes) + '\n')
        delete_file.write('rm -rf ~/.tendermint/cluster{}'.format(nodes))
    st = os.stat(delete_file_name)
    os.chmod(delete_file_name, st.st_mode | stat.S_IEXEC)


def create_reset_file(nodes):
    reset_file_name = './tools/local-cluster-reset-{}.sh'.format(nodes)
    with open(reset_file_name, 'w') as reset_file:
        reset_file.write('CWD=`dirname $0`' + '\n')

        reset_file.write('$CWD/local-cluster-delete-{}.sh'.format(nodes) + '\n')
        reset_file.write('$CWD/local-cluster-init-{}.sh'.format(nodes) + '\n')
        reset_file.write('$CWD/local-cluster-start-{}.sh'.format(nodes) + '\n')
    st = os.stat(reset_file_name)
    os.chmod(reset_file_name, st.st_mode | stat.S_IEXEC)


def main():
    args = get_args()
    create_init_file(int(args.nodes))
    create_start_file(int(args.nodes))
    create_stop_file(int(args.nodes))
    create_delete_file(args.nodes)
    create_reset_file(args.nodes)

def get_args():

    parser = argparse.ArgumentParser(
        description=HELP_MESSAGE,
        formatter_class=argparse.RawTextHelpFormatter)

    parser.add_argument('-d', '--debug',
                        default=False, action='store_true',
                        help="Output debug log to _s-tui.log")
    parser.add_argument('-n', '--nodes',
                        default=4,
                        help="Number of nodes to create configs for")
    args = parser.parse_args()
    return args


if __name__ == '__main__':
    main()
