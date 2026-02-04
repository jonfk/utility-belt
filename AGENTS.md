# utility-belt

This repository contains various programs that make life useful.

## Project Structure & Module Organization
- `go/` hosts CLI utilities in `src/github.com/jonfk/utility-belt/*`
- `js/` contains the downloader client and Fastify server
- `python/` stores standalone scripts plus `uv`-ready tools (`git-smart-commit`, `prune-openapi`)
- `rust/` tracks Cargo apps (`cmd-queue`, `move-photos-without-duplicates`)
- `firefox//` covers the firefox browser extensions
- `sh/` holds reusable shell helpers

## Installing commands
- The root `justfile` contains helpers to install utilities of this repo (script, binary, etc) into `~/.local/bin/`.
- The recipe may delegate to another recipe or script to do the actual building and install.

## utility-belt helper command
- `sh/utility-belt.sh` contains a helper script that lists installed utility-belt programs.
- When a new utility-belt program is added to the `justfile` intstaller. It shuold also be added to `sh/utility-belt.sh`
