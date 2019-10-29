echo "$0: MP-EDDSA"
#clean

rm signature*
rm log-sign*.log
rm log-error*.log

# First argument is the number fo nodes in the cluseter
n=${1:-4}
 # Second argument is the number of parties
k=${2:-4}

echo "sign part"
for i in $(seq 1 $k);
do
    S=$(( ( RANDOM % $n ) ))
    PORT=$(( 46057 + $S * 100 ))
    # cargo run -p mmpc-client --bin  sign-client -- -I $i -C $n -M "message" --proxy 127.0.0.1:$PORT &
    ./target/debug/sign-client -I $i -C $k -M "message" --proxy 127.0.0.1:$PORT &
done
