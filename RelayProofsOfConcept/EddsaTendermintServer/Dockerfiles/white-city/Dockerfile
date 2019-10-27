FROM ubuntu:latest

WORKDIR /eddsatendermint
# VOLUME [ /eddsatendermint ]

COPY ./tools/docker/wait-for-it.sh ./wait-for-it.sh
COPY ./protocols.json ./protocols.json
COPY ./target/release/server ./server
COPY ./target/release/kg-client ./kg-client
COPY ./target/release/sign-client ./sign-client

EXPOSE 26658
STOPSIGNAL SIGTERM