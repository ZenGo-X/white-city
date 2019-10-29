echo "$0: MP-EDDSA"
#clean

rm keys*
rm log-kg*.log
rm log-error*.log


# First argument is the number fo nodes in the cluseter
n=${1:-4}
 # Second argument is the number of parties
k=${2:-4}

cargo build --all

echo "keygen part"
for i in $(seq 1 $k);
do
    S=$(( ( RANDOM % $n ) ))
    PORT=$(( 46057 + $S * 100 ))
    #PORT="46157"
    # cargo run -p mmpc-client --bin kg-client -- -I $i -C $n --proxy 127.0.0.1:$PORT -v &
    #./target/debug/kg-client -I $i -C $k --proxy 127.0.0.1:$PORT &> log-error$i.log &
    ./target/debug/kg-client -I $i -C $k --proxy 127.0.0.1:$PORT &
done
