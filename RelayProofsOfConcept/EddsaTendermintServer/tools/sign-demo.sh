echo "$0: MP-EDDSA"
#clean

rm signature?
rm signature??
rm log*.log

n=4

echo "sign part"
for i in $(seq 1 $n);
do
    PORT="46${i}57"
    cargo run --example sign-client -- -I $i -P $n -M "message" &> log$i.log --proxy 127.0.0.1:$PORT &
    sleep 0.1
done
