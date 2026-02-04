#!/usr/bin/env bash
set -euo pipefail

PROG="yt-transcript"
TMPDIR_CREATED=""

usage() {
  cat <<'EOF'
Download YouTube transcripts and write a cleaned Markdown file.

Usage:
  yt-transcript [options] <youtube-url>

Options:
  -l, --lang <code>       Subtitle language(s) (default: en)
                          Examples: en, en-US, en.*, en,es
  -o, --out-dir <dir>     Output directory (default: .)
      --max-len <n>       Max kebab-title length (default: 80)
      --subtitle          Also save the raw .vtt subtitle file
      --overwrite         Overwrite existing output files (default: auto-suffix)
  -h, --help              Show this help

Output:
  Writes: <kebab-title>--<video-id>.md
  With --subtitle, also writes: <kebab-title>--<video-id>.vtt

Notes:
  - Requires yt-dlp in PATH.
  - Fetches official subtitles first, then falls back to auto subtitles.
  - Prints the written .md path to stdout (logs go to stderr).
EOF
}

log() {
  echo "$PROG: $*" >&2
}

die() {
  log "error: $*"
  exit 1
}

have() {
  command -v "$1" >/dev/null 2>&1
}

trim() {
  local s="$1"
  # shellcheck disable=SC2001
  s="$(echo "$s" | sed -E 's/^[[:space:]]+//; s/[[:space:]]+$//')"
  printf '%s' "$s"
}

slugify() {
  local title="$1"
  local max_len="$2"

  local slug=""
  if have python3; then
    slug="$(
      python3 - "$title" "$max_len" <<'PY'
import re
import sys
import unicodedata

title = sys.argv[1]
max_len = int(sys.argv[2])

s = unicodedata.normalize("NFKD", title)
s = s.encode("ascii", "ignore").decode("ascii")
s = s.lower()
s = re.sub(r"[^a-z0-9]+", "-", s).strip("-")
if not s:
    s = "video"
s = s[:max_len].rstrip("-")
print(s)
PY
    )"
  else
    slug="$(printf '%s' "$title" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g; s/^-+|-+$//g' | cut -c "1-$max_len")"
    slug="${slug%-}"
    [[ -n "$slug" ]] || slug="video"
  fi

  printf '%s\n' "$slug"
}

pick_best_vtt() {
  local requested_langs="$1"
  shift
  local -a files=("$@")

  if [[ ${#files[@]} -eq 1 ]]; then
    printf '%s\n' "${files[0]}"
    return 0
  fi

  local -a langs=()
  local IFS=,
  read -r -a langs <<<"$requested_langs"

  local lang
  for lang in "${langs[@]}"; do
    lang="$(trim "$lang")"
    [[ -n "$lang" ]] || continue
    if [[ "$lang" == *"*"* || "$lang" == *"?"* || "$lang" == *"["* ]]; then
      continue
    fi
    local f
    for f in "${files[@]}"; do
      local base
      base="$(basename "$f")"
      if [[ "$base" == *".${lang}.vtt" ]]; then
        printf '%s\n' "$f"
        return 0
      fi
    done
  done

  printf '%s\n' "${files[0]}"
}

download_subtitles() {
  local kind="$1" # official|auto
  local url="$2"
  local lang="$3"
  local dir="$4"

  mkdir -p "$dir"

  local -a args=(
    --no-playlist
    --skip-download
    --sub-langs "$lang"
    --sub-format "vtt"
    --paths "$dir"
  )

  case "$kind" in
    official)
      args+=(--write-subs)
      ;;
    auto)
      args+=(--write-auto-subs)
      ;;
    *)
      die "internal: unknown subtitle kind: $kind"
      ;;
  esac

  if ! yt-dlp "${args[@]}" "$url"; then
    return 2
  fi

  local -a candidates=()
  while IFS= read -r -d '' f; do
    candidates+=("$f")
  done < <(find "$dir" -type f -name '*.vtt' -print0)

  if [[ ${#candidates[@]} -eq 0 ]]; then
    return 1
  fi

  pick_best_vtt "$lang" "${candidates[@]}"
}

write_markdown() {
  local vtt_path="$1"
  local md_path="$2"
  local title="$3"
  local url="$4"
  local lang="$5"
  local captions_kind="$6" # official|auto

  local dir
  dir="$(dirname "$md_path")"
  mkdir -p "$dir"

  {
    printf '# %s\n\n' "$title"
    printf 'Source: %s\n\n' "$url"
    printf 'Language: %s\n' "$lang"
    printf 'Captions: %s\n\n' "$captions_kind"
    printf -- '---\n\n'

    awk '
      BEGIN { in_header=1; in_note=0; cue=""; prev=""; first=1 }
      function flush() {
        if (cue == "") return
        if (cue == prev) { cue=""; return }
        if (!first) printf " "
        printf "%s", cue
        prev = cue
        cue=""
        first=0
      }
      {
        line=$0
        sub(/\r$/, "", line)
        stripped=line
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", stripped)

        if (in_header) {
          if (stripped == "") in_header=0
          next
        }

        if (stripped ~ /^NOTE/) { in_note=1; next }
        if (in_note) {
          if (stripped == "") in_note=0
          next
        }

        if (line ~ /-->/) { flush(); next }
        if (stripped == "") { flush(); next }
        if (cue == "" && stripped ~ /^[0-9]+$/) next

        gsub(/<[^>]+>/, "", line)
        gsub(/&amp;/, "\\&", line)
        gsub(/&lt;/, "<", line)
        gsub(/&gt;/, ">", line)
        gsub(/&quot;/, "\"", line)
        gsub(/&#39;/, sprintf("%c", 39), line)
        gsub(/&nbsp;/, " ", line)
        gsub(/[[:space:]]+/, " ", line)
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", line)
        if (line == "") next

        if (cue != "") cue = cue " " line
        else cue = line
      }
      END { flush(); print "" }
    ' "$vtt_path" | fold -s -w 100

    printf '\n'
  } >"$md_path"
}

main() {
  local lang="en"
  local out_dir="."
  local max_len=80
  local want_subtitle=0
  local overwrite=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      -h|--help)
        usage
        exit 0
        ;;
      -l|--lang)
        [[ $# -ge 2 ]] || die "--lang requires a value"
        lang="$2"
        shift 2
        ;;
      -o|--out-dir)
        [[ $# -ge 2 ]] || die "--out-dir requires a value"
        out_dir="$2"
        shift 2
        ;;
      --max-len)
        [[ $# -ge 2 ]] || die "--max-len requires a value"
        max_len="$2"
        shift 2
        ;;
      --subtitle)
        want_subtitle=1
        shift
        ;;
      --overwrite)
        overwrite=1
        shift
        ;;
      --)
        shift
        break
        ;;
      -*)
        die "unknown option: $1 (try: --help)"
        ;;
      *)
        break
        ;;
    esac
  done

  [[ $# -eq 1 ]] || { usage >&2; exit 1; }
  local url="$1"

  if ! have yt-dlp; then
    log "yt-dlp not found in PATH"
    exit 127
  fi

  if ! [[ "$max_len" =~ ^[0-9]+$ ]] || [[ "$max_len" -le 0 ]]; then
    die "--max-len must be a positive integer"
  fi

  mkdir -p "$out_dir"

  cleanup() {
    if [[ -n "${TMPDIR_CREATED:-}" && -d "$TMPDIR_CREATED" ]]; then
      rm -rf "$TMPDIR_CREATED"
    fi
    return 0
  }
  on_interrupt() {
    log "interrupted"
    exit 130
  }
  trap cleanup EXIT
  trap on_interrupt INT

  TMPDIR_CREATED="$(mktemp -d 2>/dev/null)" || die "failed to create temporary directory"

  local meta
  if ! meta="$(yt-dlp --no-playlist --skip-download --print "%(id)s" --print "%(title)s" "$url")"; then
    die "yt-dlp failed while fetching video metadata"
  fi

  local -a meta_lines=()
  mapfile -t meta_lines <<<"$meta"
  local video_id="${meta_lines[0]:-}"
  local title="${meta_lines[1]:-}"
  [[ -n "$video_id" && -n "$title" ]] || die "failed to parse video metadata (is the URL valid?)"

  local slug
  slug="$(slugify "$title" "$max_len")"
  [[ -n "$slug" ]] || slug="$video_id"

  local base="${slug}--${video_id}"
  local final_base="$base"
  local i=1

  while [[ $overwrite -eq 0 ]]; do
    local md_candidate="$out_dir/$final_base.md"
    local vtt_candidate="$out_dir/$final_base.vtt"
    if [[ -e "$md_candidate" ]]; then
      :
    elif [[ $want_subtitle -eq 1 && -e "$vtt_candidate" ]]; then
      :
    else
      break
    fi
    i=$((i + 1))
    final_base="${base}-${i}"
  done

  local md_out="$out_dir/$final_base.md"
  local vtt_out="$out_dir/$final_base.vtt"

  local official_dir="$TMPDIR_CREATED/official"
  local auto_dir="$TMPDIR_CREATED/auto"

  local vtt_path=""
  local captions_kind=""

  log "fetching subtitles (lang: $lang)"
  if vtt_path="$(download_subtitles "official" "$url" "$lang" "$official_dir")"; then
    captions_kind="official"
  else
    local rc=$?
    if [[ $rc -eq 2 ]]; then
      die "yt-dlp failed while fetching official subtitles"
    fi
  fi

  if [[ -z "$vtt_path" ]]; then
    log "no official subtitles found; trying auto subtitles"
    if vtt_path="$(download_subtitles "auto" "$url" "$lang" "$auto_dir")"; then
      captions_kind="auto"
    else
      local rc=$?
      if [[ $rc -eq 2 ]]; then
        die "yt-dlp failed while fetching auto subtitles"
      fi
      die "no subtitles found for language '$lang'"
    fi
  fi

  if [[ $want_subtitle -eq 1 ]]; then
    cp "$vtt_path" "$vtt_out"
    log "wrote subtitle: $vtt_out"
  fi

  write_markdown "$vtt_path" "$md_out" "$title" "$url" "$lang" "$captions_kind"
  log "wrote transcript: $md_out"

  printf '%s\n' "$md_out"
}

main "$@"
