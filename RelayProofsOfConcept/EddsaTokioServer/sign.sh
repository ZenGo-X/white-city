
echo "$0: 2P-EDDSA"
#clean



kill -9 $(lsof -t -i:8080)


echo "signing part"
cargo run --package relay-server --bin server &
sleep 2
cargo run --bin eddsa_sign_client 127.0.0.1:8080 keys2 $1 &
sleep 2
cargo run --bin eddsa_sign_client 127.0.0.1:8080 keys1 $1 &











