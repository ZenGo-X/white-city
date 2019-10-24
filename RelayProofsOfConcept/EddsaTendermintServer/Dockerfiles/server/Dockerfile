# select image
FROM rust:latest

WORKDIR /eddsatendermintserver

COPY ./ ./

# this build step will cache your dependencies
RUN cargo build --release

# build for release
RUN rm ./target/release/deps/eddsatendermintserver*
RUN cargo build --release

# set the startup command to run your binary
CMD ["./target/release/eddsatendermintserver"]

# VOLUME [ / ]
# WORKDIR /
# EXPOSE 80
# COPY target/release/server /server
# CMD ["cargo", "run", "--address", "127.0.0.1:26658"]
STOPSIGNAL SIGTERM