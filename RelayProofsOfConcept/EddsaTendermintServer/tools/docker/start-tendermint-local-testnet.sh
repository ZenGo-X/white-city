CURDIR=$(pwd)
# echo "$CURDIR"

# build white-city binaries 
cargo build --all --release

# build white-city server image
docker-compose build

# if empty block
./tools/docker/gen-empty-block-cluster.py $1 $2 $3> ./tools/docker/docker-compose.yml

# if nonempty block
# ./tools/docker/gen-nonempty-block-cluster.py $1 $2 $3> ./tools/docker/docker-compose.yml

cp ./Dockerfiles/tendermint/localnode $GOPATH/src/github.com/tendermint/tendermint/networks/local/localnode/Dockerfile
cp $GOPATH/src/github.com/tendermint/tendermint/Makefile $GOPATH/src/github.com/tendermint/tendermint/Makefile.bak
cp $GOPATH/src/github.com/tendermint/tendermint/docker-compose.yml $GOPATH/src/github.com/tendermint/tendermint/docker-compose.yml.bak
cp ./tools/docker/tendermint_Makefile $GOPATH/src/github.com/tendermint/tendermint/Makefile
cp ./tools/docker/docker-compose.yml $GOPATH/src/github.com/tendermint/tendermint/docker-compose.yml

cd $GOPATH/src/github.com/tendermint/tendermint

# Build the linux binary in ./build
make build-linux

# (optionally) Build tendermint/localnode image
# it will be called by "make localnet-start" eventually, so no need to call it explicitly 
# make build-docker-localnode

sudo rm -rf ./build/node*

make NODENUM=$1 localnet-start
