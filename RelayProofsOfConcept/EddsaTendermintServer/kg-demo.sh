echo "$0: MP-EDDSA"
#clean

rm keys?
rm keys??

#kill -9 $(lsof -t -i:26657) &&
# kill -9 $(lsof -t -i:26658) &&
#tendermint unsafe_reset_all &&
#nohup tendermint node &> /dev/null &

n=50

echo "keygen part"
#cargo run --package relay-server --bin server -- -P $n&
#sleep 2
for i in $(seq 1 $n);
do
    cargo run --example keygen-client -- -I $i -P $n&
    #sleep 0.1
done

# kill -9 $(lsof -t -i:26657) && kill -9 $(lsof -t -i:26658)
