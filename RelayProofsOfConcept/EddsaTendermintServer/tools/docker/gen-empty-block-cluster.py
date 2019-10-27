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


def print_app(i, n):
    print """  app"""+str(i)+""":
    container_name: app"""+str(i)+"""
    image: "white-city.eddsatendermint"
    command: ["/server", "--address", "192.167.9."""+str(2+i)+""":26658"]
    ports:
      - \""""+str(36656+i)+""":26658"
    networks:
      localnet:
        ipv4_address: 192.167.9."""+str(2+i)+"""
"""


def print_node(i, n):
    print """  node"""+str(i)+""":
    container_name: node"""+str(i)+"""
    image: "tendermint/localnode"
    depends_on:"""
    for x in range(n):
        print """        - app"""+str(x)+""""""
    print """    command: ["node", "--proxy_app", "tcp://192.167.9."""+str(2+i)+""":26658", "kvstore"]
    ports:
      - \""""+str(26656+2*i)+"""-"""+str(26657+2*i)+""":26656-26657"
    environment:
      - ID="""+str(i)+"""
      - LOG=${LOG:-tendermint.log}
    volumes:
      - ./build:/tendermint:Z
    networks:
      localnet:
        ipv4_address: 192.167.10."""+str(2+i)+"""
"""


def main():
    print_header()
    for i in range(node_num):
        print_app(i, node_num)
    for i in range(node_num):
        print_node(i, node_num)
    print_tailer()        



if __name__ in "__main__":
    main()