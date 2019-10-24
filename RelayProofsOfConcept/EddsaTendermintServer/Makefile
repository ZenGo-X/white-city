build:
	cargo build --all

clean:
	rm log*.log
	rm relay-server*.log
	rm signature*
	rm keys*

clean-exp:
	rm exp-sign*
	rm exp-kg*

clean-all: clean clean-exp
	rm exp-full

kill:
	pgrep tendermint | xargs kill -KILL
	pgrep kg | xargs kill -KILL
	pgrep sign | xargs kill -KILL

