echo "$0: MP-EDDSA"
#clean

rm keys?
rm keys??

n=1

echo "keygen part"
for i in $(seq 1 $n);
do
    cargo run --example keygen-client -- -I $i -P $n &
    # sleep 0.1
done
