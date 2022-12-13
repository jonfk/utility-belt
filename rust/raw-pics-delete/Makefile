.PHONY: install

target/release/raw-pics-delete: $(shell find src -type f) Cargo.toml Cargo.lock
	cargo build --release

install: target/release/raw-pics-delete
	cp target/release/raw-pics-delete /usr/local/bin
