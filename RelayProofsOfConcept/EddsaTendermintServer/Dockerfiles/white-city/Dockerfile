FROM ubuntu:latest

WORKDIR /

COPY ./target/release/server /server
COPY ./target/release/kg-client /kg-client
COPY ./target/release/sign-client /sign-client

# VOLUME [ / ]
EXPOSE 26658
STOPSIGNAL SIGTERM