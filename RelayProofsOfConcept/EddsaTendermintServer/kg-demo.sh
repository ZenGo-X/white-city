echo "$0: MP-EDDSA"
#clean

rm keys?
rm keys??
rm log-kg*.log

n=32

echo "keygen part"
for i in $(seq 1 $n);
do
    #cargo run -p mmpc-client --bin kg-client -- -I $i --capacity $n -v &
    cargo run -p mmpc-client --bin kg-client -- -I $i --capacity $n &
done
