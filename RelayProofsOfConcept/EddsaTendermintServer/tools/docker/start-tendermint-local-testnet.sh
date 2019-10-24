where_am_i=$(pwd)
# echo "$where_am_i"
# cd $where_am_i

make server

# ./tools/docker/gen-docker-compose-config.py $1 > ./tools/docker/docker-compose.yml

# if not to commit empty blocks
cp ./Dockerfiles/tendermint/localnode $GOPATH/src/github.com/tendermint/tendermint/networks/local/localnode/Dockerfile
# if to commit empty blocks
cp ./Dockerfiles/tendermint/localnode_empty $GOPATH/src/github.com/tendermint/tendermint/networks/local/localnode/Dockerfile

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
