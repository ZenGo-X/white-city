CWD=`dirname $0`
$CWD/local-cluster-stop.sh

rm -rf ~/.tendermint/cluster4/1
rm -rf ~/.tendermint/cluster4/2
rm -rf ~/.tendermint/cluster4/3
rm -rf ~/.tendermint/cluster4/4
