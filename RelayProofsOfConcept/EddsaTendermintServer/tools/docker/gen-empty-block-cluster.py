#!/usr/bin/env python2
import sys
import random

NODE_NUM = int(sys.argv[1])
CLIENT_NUM = int(sys.argv[2])


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
    command: bash -c "./server --address 192.167.9."""+str(2+i)+""":26658"
    ports:
      - \""""+str(36656+i)+""":26658"
    volumes:
      - ~/eddsatendermint:/eddsatendermint/data
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


def print_kg(i, c, n):
    random.seed()
    assigned_to = random.randint(0, n-1)
    print """  kg"""+str(i)+""":
    container_name: kg"""+str(i)+"""
    image: "white-city.eddsatendermint"
    depends_on:"""
    for x in range(n):
        print """        - node"""+str(x)+""""""
    print """    command: bash -c "./wait-for-it.sh 192.167.10."""+str(assigned_to+2)+""":26657 -- ./kg-client -I """+ str(i+1) +""" -C """+str(c)+""" --proxy 192.167.10."""+str(assigned_to+2)+""":26657 && cp ./keys* /eddsatendermint/data/ && cp ./*.log /eddsatendermint/data/"
    volumes:
      - ~/eddsatendermint:/eddsatendermint/data
    networks:
      localnet:
        ipv4_address: 192.167.11."""+str(2+i)+"""
"""


def main():
    print_header()
    for i in range(NODE_NUM):
        print_app(i, NODE_NUM)
    for i in range(NODE_NUM):
        print_node(i, NODE_NUM)
    for i in range(CLIENT_NUM):
        print_kg(i, CLIENT_NUM, NODE_NUM)
    print_tailer()        



if __name__ in "__main__":
    main()