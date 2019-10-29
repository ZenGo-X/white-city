import argparse
import os
import stat
import subprocess
import time
import csv

HELP_MESSAGE = """
Run local kg and sign experiments
"""

def write_result(parties, write_filename, max_time):
    file_exists = os.path.isfile(write_filename)
    with open(write_filename, 'a') as csvfile:
        fieldnames = ["parties", "time"]
        csv_dict = {"parties": parties, "time": max_time}
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
        if not file_exists:
            writer.writeheader()  # file doesn't exist yet, write a header
        writer.writerow(csv_dict)

def get_max_run_time(parties, read_filename):
    print("################ GETTING DATA #######################")
    try:
        with open(read_filename, mode='r') as csv_file:
            times = list()
            csv_reader = csv.DictReader(csv_file)
            for row in csv_reader:
                try:
                    times.append(int(row["millis"]))
                except:
                    pass
            max_time = max(times)
            return max_time
    except:
        return 0


def main():

    def run_exps(exp_type, nodes, parties):
        max_vec = list()
        for i in range(1):
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
            sleep_time = 20
            if exp_type is "kg":
                sleep_time = max(int(int(parties)/2), 20)
            elif exp_type is "sign":
                sleep_time = max(int(int(parties) * 1.5), 20)
            time.sleep(sleep_time)
            write_filename = "./full-exp-{}-{}.csv".format(exp_type, nodes)
            val = int(get_max_run_time(parties, exp_filename))
            if val != 0:
                max_vec.append(val)
        avg = int(sum(max_vec) / len(max_vec))
        write_result(parties, write_filename, avg)

    args = get_args()
    #nodes_range = [4, 2, 1]
    nodes_range = [10]
    #parties_range = [8, 4]
    parties_range = range(60, 0, -10)
    if args.nodes:
        nodes_range = [args.nodes]
    if args.parties:
        parties_range = [args.parties]
    for nodes in nodes_range:
        for parties in parties_range:
            run_exps("kg", nodes, parties)
            run_exps("sign", nodes, parties)


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
