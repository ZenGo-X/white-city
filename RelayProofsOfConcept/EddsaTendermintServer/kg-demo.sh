echo "$0: MP-EDDSA"
#clean

rm keys*
rm log-kg*.log
rm log-error*.log

n=3

echo "keygen part"
for i in $(seq 1 $n);
do
    #cargo run -p mmpc-client --bin kg-client -- -I $i --capacity $n -v &
    cargo run -p mmpc-client --bin kg-client -- -I $i --capacity $n &
done
