echo "$0: MP-EDDSA"
#clean

rm keys?
rm keys??


n=4

echo "keygen part"
for i in $(seq 1 $n);
do
    PORT="46${i}57"
    cargo run --example keygen-client -- -I $i -P $n --proxy 127.0.0.1:$PORT &
    sleep 0.1
done
