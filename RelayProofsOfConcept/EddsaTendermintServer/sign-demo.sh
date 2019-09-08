echo "$0: MP-EDDSA"
#clean

rm signature?
rm signature??
rm log*.log

n=1

echo "sign part"
for i in $(seq 1 $n);
do
    cargo run --example sign-client -- -I $i -P $n -M "message" &> log$i.log &
done
