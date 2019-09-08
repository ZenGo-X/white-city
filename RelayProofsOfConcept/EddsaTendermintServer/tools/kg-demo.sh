echo "$0: MP-EDDSA"
#clean

rm keys?
rm keys??


n=50

echo "keygen part"
for i in $(seq 1 $n);
do
    S=$(( ( RANDOM % 4 )  + 1 ))
    PORT="46${S}57"
    #PORT="46157"
    cargo run --example keygen-client -- -I $i -P $n --proxy 127.0.0.1:$PORT &
done
