.PHONY: install

target/release/cmdq: $(shell find src -type f)
	cargo build --release

install: target/release/cmdq
	cp target/release/cmdq target/release/cmdq_server ${HOME}/bin
