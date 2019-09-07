tendermint init --home $HOME/.tendermint/cluster4/1
tendermint init --home $HOME/.tendermint/cluster4/2
tendermint init --home $HOME/.tendermint/cluster4/3
tendermint init --home $HOME/.tendermint/cluster4/4
echo "node1: `tendermint show_node_id --home $HOME/.tendermint/cluster4/1`"
echo "node2: `tendermint show_node_id --home $HOME/.tendermint/cluster4/2`"
echo "node3: `tendermint show_node_id --home $HOME/.tendermint/cluster4/3`"
echo "node4: `tendermint show_node_id --home $HOME/.tendermint/cluster4/4`"

TM_VALIDATOR1='{"pub_key":'$(tendermint show_validator --home $HOME/.tendermint/cluster4/1)',"power":10,"name":""}'
TM_VALIDATOR2='{"pub_key":'$(tendermint show_validator --home $HOME/.tendermint/cluster4/2)',"power":10,"name":""}'
TM_VALIDATOR3='{"pub_key":'$(tendermint show_validator --home $HOME/.tendermint/cluster4/3)',"power":10,"name":""}'
TM_VALIDATOR4='{"pub_key":'$(tendermint show_validator --home $HOME/.tendermint/cluster4/4)',"power":10,"name":""}'
TM_VALIDATORS=$TM_VALIDATOR1,$TM_VALIDATOR2,$TM_VALIDATOR3,$TM_VALIDATOR4
sed -i -e 's#'$TM_VALIDATOR1'#'$TM_VALIDATORS'#g' $HOME/.tendermint/cluster4/1/config/genesis.json
sed -i -e 's#'$TM_VALIDATOR1'#'$TM_VALIDATORS'#g' $HOME/.tendermint/cluster4/2/config/genesis.json
sed -i -e 's#'$TM_VALIDATOR1'#'$TM_VALIDATORS'#g' $HOME/.tendermint/cluster4/3/config/genesis.json
sed -i -e 's#'$TM_VALIDATOR1'#'$TM_VALIDATORS'#g' $HOME/.tendermint/cluster4/4/config/genesis.json

sed -i -e 's#addr_book_strict = true#addr_book_strict = false#g' $HOME/.tendermint/cluster4/1/config/config.toml
sed -i -e 's#addr_book_strict = true#addr_book_strict = false#g' $HOME/.tendermint/cluster4/2/config/config.toml
sed -i -e 's#addr_book_strict = true#addr_book_strict = false#g' $HOME/.tendermint/cluster4/3/config/config.toml
sed -i -e 's#addr_book_strict = true#addr_book_strict = false#g' $HOME/.tendermint/cluster4/4/config/config.toml
