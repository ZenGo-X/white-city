import argparse
import os
import stat
import subprocess

HELP_MESSAGE = """
Run local kg and sign experiments
"""


def main():
    args = get_args()
    for nodes in [1, 2, 4, 8, 16, 32]:
        subprocess.call(["python", "generate.py", nodes])
        subprocess.call(['./tools/local-cluster-start{}.sh'.format(nodes)])


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
