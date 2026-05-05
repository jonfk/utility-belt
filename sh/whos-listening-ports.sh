#!/usr/bin/env bash
set -euo pipefail

if [[ "${EUID}" -eq 0 ]]; then
  min_port="${1:-0}"
else
  min_port="${1:-1024}"
fi

user_width=12
command_width=20
fixed_width=$((8 + 1 + 8 + 1 + user_width + 1 + command_width + 1))

terminal_width=""
if [[ "${COLUMNS:-}" =~ ^[0-9]+$ ]]; then
  terminal_width="$COLUMNS"
elif [[ -t 0 ]] && terminal_size="$(stty size 2>/dev/null)"; then
  terminal_width="$(awk '{ print $2 }' <<<"$terminal_size")"
elif [[ -t 1 ]]; then
  terminal_width="$(tput cols 2>/dev/null || true)"
fi

max_args_width=0
if [[ "$terminal_width" =~ ^[0-9]+$ ]]; then
  max_args_width=$((terminal_width - fixed_width))
  if (( max_args_width < 40 )); then
    max_args_width=40
  fi
fi

truncate_middle() {
  local value="$1"
  local max_width="$2"
  local marker="..."
  local marker_width="${#marker}"

  if (( max_width == 0 )); then
    printf "%s" "$value"
    return
  fi

  if (( ${#value} <= max_width )); then
    printf "%s" "$value"
    return
  fi

  local keep_width=$((max_width - marker_width))
  local start_width=$(((keep_width + 1) / 2))
  local end_width=$((keep_width / 2))

  printf "%s%s%s" "${value:0:start_width}" "$marker" "${value: -end_width}"
}

printf "%-8s %-8s %-*s %-*s %s\n" "PORT" "PID" "$user_width" "USER" "$command_width" "COMMAND" "PATH / ARGS"

lsof -nP -iTCP -sTCP:LISTEN |
awk -v min_port="$min_port" '
  NR > 1 {
    name = $(NF - 1)
    if (match(name, /:[0-9]+$/)) {
      port = substr(name, RSTART + 1, RLENGTH - 1)
      if (port >= min_port) {
        print port, $2, $3, $1
      }
    }
  }
' |
sort -n |
while read -r port pid user command; do
  args="$(ps -p "$pid" -o command= 2>/dev/null || true)"
  command="$(truncate_middle "$command" "$command_width")"
  args="$(truncate_middle "$args" "$max_args_width")"
  printf "%-8s %-8s %-*s %-*s %s\n" "$port" "$pid" "$user_width" "$user" "$command_width" "$command" "$args"
done
