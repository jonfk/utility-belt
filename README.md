# utility-belt
A collection of various programs that make life useful.

## Installing commands
Active install recipes live in the root `justfile`.

The flake exports `git-smart-push` for `aarch64-darwin` and `x86_64-linux`.
Its package owns mandatory runtime dependencies; optional desktop integration
is supplied by the host through `GIT_OPEN_CMD`. See the
[command documentation](python/git-smart-push/README.md).

Consumers should pin this flake in their lockfile. Run `nix flake check` on each
supported platform before publishing package changes.

## Deprecated tools
Deprecated tools are moved under `deprecated/` and are intentionally excluded from the main install and `utility-belt` flows.

Install archived tools explicitly from the deprecated justfile:

```sh
just --justfile deprecated/justfile install-git-smart-commit
```

## Go
Go is setup with glide for vendoring and needs to be symlinked to the GOPATH to build programs.
A standard Go dev setup is expected.
