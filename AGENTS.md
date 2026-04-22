# utility-belt

This repository contains various programs that make life useful.

## Project Structure & Module Organization
- `deprecated/` stores archived programs and their separate install recipes
- `go/` hosts CLI utilities in `src/github.com/jonfk/utility-belt/*`
- `js/` contains the downloader client and Fastify server
- `python/` stores standalone scripts plus active `uv`-ready tools (`prune-openapi`, etc.)
- `rust/` tracks Cargo apps (`cmd-queue`, `move-photos-without-duplicates`)
- `firefox//` covers the firefox browser extensions
- `sh/` holds reusable shell helpers

## Installing commands
- The root `justfile` contains helpers to install active utilities of this repo (script, binary, etc) into `~/.local/bin/`.
- Deprecated utilities should be installed from `deprecated/justfile`, not re-added to the root `justfile`.
- The recipe may delegate to another recipe or script to do the actual building and install.

## utility-belt helper command
- `sh/utility-belt.sh` contains a helper script that lists and installs active utility-belt programs.
- When a new active utility-belt program is added to the root `justfile` installer, it should also be added to `sh/utility-belt.sh`.
