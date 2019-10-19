./tools/docker/gen-docker-compose-config.py $1 > ./tools/docker/docker-compose.yml

cp $GOPATH/src/github.com/tendermint/tendermint/Makefile $GOPATH/src/github.com/tendermint/tendermint/Makefile.bak
cp $GOPATH/src/github.com/tendermint/tendermint/docker-compose.yml $GOPATH/src/github.com/tendermint/tendermint/docker-compose.yml.bak
cp ./tools/docker/tendermint_Makefile $GOPATH/src/github.com/tendermint/tendermint/Makefile
cp ./tools/docker/docker-compose.yml $GOPATH/src/github.com/tendermint/tendermint/docker-compose.yml

cd $GOPATH/src/github.com/tendermint/tendermint

# Build the linux binary in ./build
make build-linux

# (optionally) Build tendermint/localnode image
make build-docker-localnode

sudo rm -rf ./build/node*

make NODENUM=$1 localnet-start
