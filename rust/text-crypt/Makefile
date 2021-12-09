.PHONY: install

target/release/text-crypt:
	cargo build --release

install: target/release/text-crypt
	cp target/release/text-crypt ${HOME}/bin
