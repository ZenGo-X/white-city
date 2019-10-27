# README

## Requirements

Follows https://tendermint.com/docs/networks/docker-compose.html#requirements

+ Install tendermint
+ Install docker
+ Install docker-compose
    * https://stackoverflow.com/questions/48957195/how-to-fix-docker-got-permission-denied-issue

## Run

Assuming 3 servers, and 5 clients:

To DKG:

```
./tools/docker/start-tendermint-local-testnet.sh 3 5 kg
```

The keys and logs can then be found in `~/eddsatendermint/`.

Then simply run

```
./tools/docker/start-tendermint-local-testnet.sh 3 5 sign
```
