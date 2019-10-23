import argparse
import os
import stat
import subprocess
import time
import csv

HELP_MESSAGE = """
Run local kg and sign experiments
"""

def get_max_run_time(parties, read_filename, write_filename):
    with open(read_filename, mode='r') as csv_file:
        times = list()
        csv_reader = csv.DictReader(csv_file)
        for row in csv_reader:
            times.append(row["millis"])
        max_time = max(times)

    file_exists = os.path.isfile(write_filename)
    with open(write_filename, 'a') as csvfile:
        fieldnames = ["parties", "time"]
        csv_dict = {"parties": parties, "time": max_time}
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
        if not file_exists:
            writer.writeheader()  # file doesn't exist yet, write a header
        writer.writerow(csv_dict)

def main():

    def run_exps(exp_type, nodes, parties):
        exp_filename = "./exp-{}-{}.csv".format(exp_type, parties)
        try:
            os.remove(exp_filename)
        except:
            pass
        subprocess.call(["python", "generate.py", "-n", str(nodes)])
        reset_tool = "./tools/local-cluster-reset-{}.sh".format(nodes)
        subprocess.call(["sh", reset_tool])
        sleep_time = max(int(nodes), 10)
        # Give time for all nodes to connect
        time.sleep(sleep_time)
        tool = "./tools/{}-demo.sh".format(exp_type)
        subprocess.call(["sh", tool, str(nodes), str(parties)])
        time.sleep(parties)
        write_filename = "./full-exp-{}-{}.csv".format(exp_type, nodes)
        get_max_run_time(parties, exp_filename, write_filename)

    args = get_args()
    nodes_range = [1, 2, 4]
    parties_range = [4, 8]
    if args.nodes:
        nodes_range = [args.nodes]
    if args.parties:
        parties_range = [args.parteis]
    for nodes in nodes_range:
        for parties in parties_range:
            run_exps("kg", nodes, parties)


def get_args():

    parser = argparse.ArgumentParser(
        description=HELP_MESSAGE,
        formatter_class=argparse.RawTextHelpFormatter)

    parser.add_argument('-d', '--debug',
                        default=False, action='store_true',
                        help="Output debug log to _s-tui.log")
    parser.add_argument('-n', '--nodes',
                        default=None,
                        help="Number of nodes to create configs for")
    parser.add_argument('-p', '--parties',
                        default=None,
                        help="Number of nodes to create configs for")
    args = parser.parse_args()
    return args


if __name__ == '__main__':
    main()
