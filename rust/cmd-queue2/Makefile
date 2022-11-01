.PHONY: install uninstall

target/release/cmdq2: $(shell find src -type f)
	cargo build --release

install: target/release/cmdq2
	cp target/release/cmdq2 /usr/local/bin
	# cmdq generate-completion zsh > /usr/local/share/zsh/site-functions/_cmdq

uninstall:
	rm /usr/local/bin/cmdq2
