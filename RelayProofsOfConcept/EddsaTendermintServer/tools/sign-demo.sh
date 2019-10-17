echo "$0: MP-EDDSA"
#clean

rm signature?
rm signature??
rm log*.log

n=5

echo "sign part"
for i in $(seq 1 $n);
do
    S=$(( ( RANDOM % 4 )  + 1 ))
    PORT="46${S}57"
    #PORT="46157"
    # cargo run -p mmpc-client --bin  sign-client -- -I $i -C $n -M "message" --proxy 127.0.0.1:$PORT &
    ./target/debug/sign-client -I $i -C $n -M "message" --proxy 127.0.0.1:$PORT &
done
