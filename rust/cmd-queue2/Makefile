.PHONY: install

target/release/cmdq2: $(shell find src -type f)
	cargo build --release

install: target/release/cmdq2
	cp target/release/cmdq2 target/release/cmdq_server /usr/local/bin
	# cmdq generate-completion zsh > /usr/local/share/zsh/site-functions/_cmdq
