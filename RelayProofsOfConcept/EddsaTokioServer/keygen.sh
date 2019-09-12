
echo "$0: MP-EDDSA"
#clean


rm keys??

kill -9 $(lsof -t -i:8080)

n=2

echo "keygen part"
cargo run --package relay-server --bin server -- -P $n &
sleep 2
for i in $(seq 1 $n);
do
    cargo run --example eddsa_key_gen_client -- 127.0.0.1:8080 "keys$i" -P $n &
    sleep 1
done
