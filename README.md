# utility-belt
A collection of various programs that make life useful.

## Installing commands
Active install recipes live in the root `justfile`.

## Deprecated tools
Deprecated tools are moved under `deprecated/` and are intentionally excluded from the main install and `utility-belt` flows.

Install archived tools explicitly from the deprecated justfile:

```sh
just --justfile deprecated/justfile install-git-smart-commit
```

## Go
Go is setup with glide for vendoring and needs to be symlinked to the GOPATH to build programs.
A standard Go dev setup is expected.
