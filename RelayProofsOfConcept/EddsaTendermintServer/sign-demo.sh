echo "$0: MP-EDDSA"
#clean

rm signature?
rm signature??

#kill -9 $(lsof -t -i:26657) &&
# kill -9 $(lsof -t -i:26658) &&
#tendermint unsafe_reset_all &&
#nohup tendermint node &> /dev/null &

n=20

echo "sign part"
#cargo run --package relay-server --bin server -- -P $n&
#sleep 2
for i in $(seq 1 $n);
do
    cargo run --example sign-client -- -I $i -P $n &> log$i.log &
done

# kill -9 $(lsof -t -i:26657) && kill -9 $(lsof -t -i:26658)
