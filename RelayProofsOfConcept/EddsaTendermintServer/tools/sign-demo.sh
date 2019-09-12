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
    cargo run --example sign-client -- -I $i -P $n -M "message" &> log$i.log --proxy 127.0.0.1:$PORT &
done
