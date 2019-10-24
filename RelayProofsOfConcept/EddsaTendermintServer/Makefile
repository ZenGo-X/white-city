# Makefile for the "white-city server" docker image.

binaries:
	cargo build --all

server-docker: binaries
	docker build --tag white-city/server Dockerfiles/server

.PHONY: server-docker

