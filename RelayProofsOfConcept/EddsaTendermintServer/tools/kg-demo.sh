echo "$0: MP-EDDSA"
#clean

rm keys?
rm keys??
rm log-kg*.log
rm log-error*.log


n=5

cargo build --all

echo "keygen part"
for i in $(seq 1 $n);
do
    S=$(( ( RANDOM % 4 )  + 1 ))
    PORT="46${S}57"
    #PORT="46157"
    # cargo run -p mmpc-client --bin kg-client -- -I $i -C $n --proxy 127.0.0.1:$PORT -v &
    ./target/debug/kg-client -I $i -C $n --proxy 127.0.0.1:$PORT &> log-error$i.log &
done
