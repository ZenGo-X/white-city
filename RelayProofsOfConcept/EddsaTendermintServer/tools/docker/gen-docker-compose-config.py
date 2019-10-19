#!/usr/bin/env python2
import sys
node_num = int(sys.argv[1])


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


def print_node(i):
    print """  node"""+str(i)+""":
    container_name: node"""+str(i)+"""
    image: "tendermint/localnode"
    ports:
      - \""""+str(26656+2*i+(1 if i>0 else 0))+"""-"""+str(26657+2*i+(1 if i>0 else 0))+""":26656-26657"
    environment:
      - ID="""+str(i)+"""
      - LOG=${LOG:-tendermint.log}
    volumes:
      - ./build:/tendermint:Z
    networks:
      localnet:
        ipv4_address: 192.167.10.2
"""


def main():
    print_header()
    for i in range(node_num):
        print_node(i)
    print_tailer()        



if __name__ in "__main__":
    main()