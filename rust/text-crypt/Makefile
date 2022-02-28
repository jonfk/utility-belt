.PHONY: install

target/release/text-crypt: $(shell find src -type f)
	cargo build --release

install: target/release/text-crypt
	cp target/release/text-crypt /usr/local/bin
