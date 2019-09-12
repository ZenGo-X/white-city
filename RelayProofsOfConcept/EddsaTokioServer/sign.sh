echo "$0: MP-EDDSA"
#clean

rm signature??

kill -9 $(lsof -t -i:8080)

n=2

echo "signing part"
cargo run --package relay-server --bin server -- -P $n&
sleep 2
for i in $(seq 1 $n);
do
    cargo run --example eddsa_sign_client -- 127.0.0.1:8080 "keys$i" -P $n $1 &
    sleep 1
done
