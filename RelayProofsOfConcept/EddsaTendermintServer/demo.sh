echo "$0: MP-EDDSA"
#clean

rm signature??

#kill -9 $(lsof -t -i:26657) &&
# kill -9 $(lsof -t -i:26658) &&
#tendermint unsafe_reset_all &&
#nohup tendermint node &> /dev/null &

n=2

echo "signing part"
cargo run --package relay-server --bin server -- -P $n&
sleep 2
for i in $(seq 1 $n);
do
    cargo run --example simple-client -- -I $i &
    sleep 2
done

kill -9 $(lsof -t -i:26657) && kill -9 $(lsof -t -i:26658)
