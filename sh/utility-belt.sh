#!/usr/bin/env bash
set -euo pipefail

UTILITY_BELT_PROGRAMS=(
  beautiful-mermaid
  git-smart-commit
  git-worktree-utils
  organize-files-into-dirs-by-date
  prune-openapi
  start-ssh-proxy
  sync-github-keys
  yt-transcript
)

usage() {
  cat <<'EOF'
utility-belt: list, run, and install utility-belt programs

Usage:
  utility-belt list
  utility-belt run
  utility-belt install [--all] [--reinstall] [<program>...]
  utility-belt activate zsh

Notes:
  - Only the programs supported by this script are shown/installed.
  - "run" only offers programs available in PATH.
EOF
}

die() {
  echo "utility-belt: $*" >&2
  exit 1
}

have() {
  command -v "$1" >/dev/null 2>&1
}

script_realpath() {
  # Resolve symlinks without relying on readlink -f (not available on macOS).
  local path="$1"
  while [ -L "$path" ]; do
    local target
    target="$(readlink "$path")" || break
    if [[ "$target" = /* ]]; then
      path="$target"
    else
      path="$(cd "$(dirname "$path")" && pwd -P)/$target"
    fi
  done
  echo "$path"
}

repo_root() {
  local src
  src="$(script_realpath "${BASH_SOURCE[0]}")"
  local dir
  dir="$(cd "$(dirname "$src")" && pwd -P)"
  (cd "$dir/.." && pwd -P)
}

is_supported_program() {
  local needle="$1"
  local p
  for p in "${UTILITY_BELT_PROGRAMS[@]}"; do
    if [[ "$p" == "$needle" ]]; then
      return 0
    fi
  done
  return 1
}

program_status() {
  # Prints: "<status>\t<path>"
  local name="$1"

  local resolved
  resolved="$(command -v "$name" 2>/dev/null || true)"
  if [[ -n "$resolved" ]]; then
    printf 'on-path\t%s\n' "$resolved"
    return 0
  fi

  local local_bin="$HOME/.local/bin/$name"
  if [[ -e "$local_bin" ]]; then
    printf 'local-bin\t%s\n' "$local_bin"
    return 0
  fi

  printf 'missing\t\n'
}

cmd_list() {
  if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
    usage
    return 0
  fi

  printf '%-34s %-10s %s\n' "NAME" "STATUS" "PATH"
  local name
  for name in "${UTILITY_BELT_PROGRAMS[@]}"; do
    local status_line status path
    status_line="$(program_status "$name")"
    status="${status_line%%$'\t'*}"
    path="${status_line#*$'\t'}"
    printf '%-34s %-10s %s\n' "$name" "$status" "$path"
  done
}

cmd_run() {
  if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
    usage
    return 0
  fi
  if [[ $# -ne 0 ]]; then
    die "run takes no arguments"
  fi
  have fzf || die "fzf is required for 'run' (install fzf and ensure it's in PATH)"

  local runnable=()
  local name
  for name in "${UTILITY_BELT_PROGRAMS[@]}"; do
    if have "$name"; then
      runnable+=("$name")
    fi
  done

  if [[ ${#runnable[@]} -eq 0 ]]; then
    die "no supported programs found in PATH (try: utility-belt install --all)"
  fi

  local selection
  selection="$(
    printf '%s\n' "${runnable[@]}" | fzf --prompt "utility-belt> " --height 40%
  )" || return 0

  [[ -n "$selection" ]] || return 0
  printf '%s\n' "$selection"
}

run_just_install() {
  local recipe="$1"
  have just || die "just is required for 'install' (https://github.com/casey/just)"

  local root
  root="$(repo_root)"
  [[ -f "$root/justfile" ]] || die "could not find justfile at: $root/justfile"

  (cd "$root" && just "$recipe")
}

cmd_install() {
  local all=0
  local reinstall=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --all)
        all=1
        shift
        ;;
      --reinstall)
        reinstall=1
        shift
        ;;
      --help|-h)
        usage
        return 0
        ;;
      --*)
        die "unknown option: $1"
        ;;
      *)
        break
        ;;
    esac
  done

  local to_install=()
  if [[ $all -eq 1 ]]; then
    to_install=("${UTILITY_BELT_PROGRAMS[@]}")
  elif [[ $# -gt 0 ]]; then
    local name
    for name in "$@"; do
      is_supported_program "$name" || die "unknown program: $name"
      to_install+=("$name")
    done
  else
    have fzf || die "provide program names or install fzf for interactive selection"

    local missing=()
    local name
    for name in "${UTILITY_BELT_PROGRAMS[@]}"; do
      if ! have "$name"; then
        missing+=("$name")
      fi
    done

    if [[ ${#missing[@]} -eq 0 ]]; then
      echo "utility-belt: all supported programs are already in PATH"
      return 0
    fi

    local selections
    selections="$(
      printf '%s\n' "${missing[@]}" | fzf --multi --prompt "Install> " --height 40%
    )" || return 0

    while IFS= read -r line; do
      [[ -n "$line" ]] || continue
      to_install+=("$line")
    done <<<"$selections"
  fi

  local name
  for name in "${to_install[@]}"; do
    if have "$name" && [[ $reinstall -eq 0 ]]; then
      echo "utility-belt: skipping $name (already in PATH; pass --reinstall to force)"
      continue
    fi
    echo "utility-belt: installing $name"
    run_just_install "install-$name"
  done
}

cmd_activate_zsh() {
  cat <<'EOF'
# utility-belt zsh integration
# Usage: eval "$(utility-belt activate zsh)"

utility-belt-run() {
  local selected
  selected="$(command utility-belt run)" || return 0
  [[ -n "$selected" ]] || return 0

  if [[ -n "${ZLE-}" ]]; then
    LBUFFER+="${selected} "
    zle redisplay
  else
    print -z -- "${selected} "
  fi
}

zle -N utility-belt-run 2>/dev/null || true
EOF
}

main() {
  local cmd="${1:-}"
  case "$cmd" in
    ""|-h|--help|help)
      usage
      ;;
    list)
      shift
      cmd_list "$@"
      ;;
    run)
      shift
      cmd_run "$@"
      ;;
    install)
      shift
      cmd_install "$@"
      ;;
    activate)
      shift
      case "${1:-}" in
        zsh)
          shift
          [[ $# -eq 0 ]] || die "activate zsh takes no extra arguments"
          cmd_activate_zsh
          ;;
        ""|-h|--help)
          usage
          ;;
        *)
          die "unknown activate target: ${1:-}"
          ;;
      esac
      ;;
    *)
      die "unknown command: $cmd"
      ;;
  esac
}

main "$@"
