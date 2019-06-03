**To Run 2p-EdDSA** 

1. Run the server : `cargo run --package relay-server --bin server`

2. Run keygen: `cargo run --bin eddsa_key_gen_client 127.0.0.1:8080 keys1` where keys1 is the party output keys
you should take `apk` for the public key to generate to address from. pay attention to use `keys2` when you run the second instance 
(you can choose different names instead of `keys1` and `keys2` )

3. Run signing: `cargo run --bin eddsa_sign_client 127.0.0.1:8080 keys1 message`
where `message` is the message to sign. Run another instance for the second party with `keys2`

4. the output will be a file with (R,s). the file is called `signature`

alternatively, run `./keygen.sh` for keygen and  `./sign.sh message` where `message` is the message to sign
