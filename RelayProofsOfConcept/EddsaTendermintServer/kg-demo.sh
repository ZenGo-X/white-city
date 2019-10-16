echo "$0: MP-EDDSA"
#clean

rm keys?
rm keys??

n=3

echo "keygen part"
for i in $(seq 1 $n);
do
    cargo run -p mmpc-client --bin keygen-client -- -I $i -P $n &
done
