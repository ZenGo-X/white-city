build:
	cargo build --all

clean:
	rm log*.log
	rm relay-server*.log
	rm keys*
	rm signature*

clean-exp:
	rm exp-kg*
	rm exp-sign*

clean-all: clean clean-exp
	rm full-exp*

kill:
	pgrep tendermint | xargs kill -KILL
	pgrep sign | xargs kill -KILL
	pgrep kg | xargs kill -KILL

