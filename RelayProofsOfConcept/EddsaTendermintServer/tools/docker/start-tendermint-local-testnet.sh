cp ./tools/docker/tendermint_Makefile $GOPATH/src/github.com/tendermint/tendermint/Makefile

cd $GOPATH/src/github.com/tendermint/tendermint

# Build the linux binary in ./build
make build-linux

# (optionally) Build tendermint/localnode image
make build-docker-localnode

rm -rf ./build/node*

make NODENUM=5 localnet-start