#!/usr/bin/env python2
import sys
node_num = sys.argv[1]


def print_header():
    print """version: '3'

services:"""


def print_tailer():
    print """networks:
  localnet:
    driver: bridge
    ipam:
      driver: default
      config:
      -
        subnet: 192.167.10.0/16

"""


def main():
    print_header()
    print_tailer()        



if __name__ in "__main__":
    main()