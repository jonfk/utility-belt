#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${HOME:-}" ]]; then
  mkdir -p "$HOME"
fi

mkdir -p "${NPM_CONFIG_CACHE:-/tmp/pi-docker-npm-cache}" "${XDG_CACHE_HOME:-/tmp/pi-docker-cache}"
if [[ -n "${NPM_CONFIG_PREFIX:-}" ]]; then
  mkdir -p "$NPM_CONFIG_PREFIX/bin" "$NPM_CONFIG_PREFIX/lib/node_modules"
fi

if [[ -t 1 && "${PI_DOCKER_SET_TITLE:-1}" != "0" ]]; then
  printf '\033]0;pi-docker: %s\007' "${PWD:-/workspace}"
fi

if [[ -t 1 && "${PI_DOCKER_BANNER:-1}" != "0" ]]; then
  printf '[pi-docker] image=%s cwd=%s\n' "${PI_DOCKER_IMAGE:-unknown}" "${PWD:-/workspace}"
fi

if [[ "${PI_DOCKER_OLLAMA_BRIDGE:-1}" != "0" ]]; then
  # Preserve host configs that use http://127.0.0.1:11434 from inside Docker.
  socat TCP-LISTEN:11434,bind=127.0.0.1,fork,reuseaddr TCP:host.docker.internal:11434 >/tmp/pi-docker-ollama-socat.log 2>&1 &
fi

if [[ $# -gt 0 && "$1" == "shell" ]]; then
  shift
  exec "${SHELL:-/bin/bash}" "$@"
fi

exec pi "$@"
