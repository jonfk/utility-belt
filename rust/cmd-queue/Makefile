.PHONY: install

target/release/cmdq: $(shell find src -type f)
	cargo build --release

install: target/release/cmdq
	cp target/release/cmdq target/release/cmdq_server /usr/local/bin
	# cmdq generate-completion zsh > /usr/local/share/zsh/site-functions/_cmdq
