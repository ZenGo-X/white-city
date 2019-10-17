echo "$0: MP-EDDSA"
#clean

rm signature?
rm signature??
rm log*.log

n=3

echo "sign part"
for i in $(seq 1 $n);
do
    cargo run -p mmpc-client --bin sign-client -- -I $i -C $n -M "message"&
done
