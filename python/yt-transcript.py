#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.9"
# ///

from __future__ import annotations

import argparse
import json
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import unicodedata
from pathlib import Path
from typing import Any, Iterable, Mapping, Optional, Sequence


PROG = "yt-transcript"

TAG_RE = re.compile(r"<[^>]*>")
SPACE_RE = re.compile(r"[ \t]+")


class UserFacingError(RuntimeError):
    pass


def log(message: str) -> None:
    print(f"{PROG}: {message}", file=sys.stderr)


def require_binary(name: str) -> None:
    if shutil.which(name) is None:
        raise UserFacingError(f"{name} not found in PATH")


def run_command(
    command: Sequence[str],
    *,
    capture_output: bool = False,
) -> subprocess.CompletedProcess[str]:
    if capture_output:
        return subprocess.run(
            list(command),
            text=True,
            stdout=subprocess.PIPE,
            stderr=sys.stderr,
            check=False,
        )

    return subprocess.run(
        list(command),
        text=True,
        stdout=sys.stderr,
        stderr=sys.stderr,
        check=False,
    )


def split_csv(raw: str) -> list[str]:
    return [part.strip() for part in raw.split(",") if part.strip()]


def slugify(title: str, max_len: int) -> str:
    value = unicodedata.normalize("NFKD", title)
    value = value.encode("ascii", "ignore").decode("ascii")
    value = value.lower()
    value = re.sub(r"[^a-z0-9]+", "-", value).strip("-")
    if not value:
        value = "video"
    value = value[:max_len].rstrip("-")
    if not value:
        value = "video"
    return value


def parse_json_stdout(raw_stdout: str) -> Mapping[str, Any]:
    text = raw_stdout.strip()
    if not text:
        raise UserFacingError("yt-dlp returned no metadata output")
    try:
        value = json.loads(text)
    except json.JSONDecodeError as exc:
        raise UserFacingError("failed to parse yt-dlp metadata JSON") from exc
    if not isinstance(value, dict):
        raise UserFacingError("unexpected yt-dlp metadata shape")
    return value


def fetch_video_info(url: str) -> Mapping[str, Any]:
    result = run_command(
        [
            "yt-dlp",
            "--no-playlist",
            "--skip-download",
            "--dump-single-json",
            url,
        ],
        capture_output=True,
    )
    if result.returncode != 0:
        raise UserFacingError("yt-dlp failed while fetching video metadata")
    return parse_json_stdout(result.stdout)


def normalize_caption_map(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        return {}
    normalized: dict[str, Any] = {}
    for key, entry in value.items():
        if isinstance(key, str):
            normalized[key] = entry
    return normalized


def langs_starting_with(langs: Iterable[str], prefix: str) -> list[str]:
    needle = prefix.lower()
    return sorted((lang for lang in langs if lang.lower().startswith(needle)), key=str.casefold)


def build_caption_attempts(info: Mapping[str, Any]) -> tuple[list[tuple[str, str]], list[str], list[str], list[str]]:
    official_langs = sorted(normalize_caption_map(info.get("subtitles")).keys(), key=str.casefold)
    auto_langs = sorted(normalize_caption_map(info.get("automatic_captions")).keys(), key=str.casefold)

    attempts: list[tuple[str, str]] = []
    step_logs: list[str] = []
    seen: set[tuple[str, str]] = set()

    def add_attempts(kind: str, label: str, langs: Iterable[str]) -> None:
        added: list[str] = []
        for lang in langs:
            key = (kind, lang)
            if key in seen:
                continue
            seen.add(key)
            attempts.append(key)
            added.append(lang)
        added_text = ", ".join(added) if added else "(none)"
        step_logs.append(f"{label}: {added_text}")

    add_attempts("official", "en official", langs_starting_with(official_langs, "en"))
    add_attempts("auto", "en auto", langs_starting_with(auto_langs, "en"))
    add_attempts("official", "fr official", langs_starting_with(official_langs, "fr"))
    add_attempts("auto", "fr auto", langs_starting_with(auto_langs, "fr"))
    add_attempts("official", "any official", official_langs)
    add_attempts("auto", "any auto", auto_langs)

    return attempts, official_langs, auto_langs, step_logs


def pick_best_vtt(requested_langs: str, files: Sequence[Path]) -> Path:
    if len(files) == 1:
        return files[0]

    for lang in split_csv(requested_langs):
        if "*" in lang or "?" in lang or "[" in lang:
            continue
        suffix = f".{lang}.vtt"
        for file_path in files:
            if file_path.name.endswith(suffix):
                return file_path
    return files[0]


def download_subtitles(kind: str, url: str, lang: str, directory: Path) -> tuple[int, Optional[Path]]:
    directory.mkdir(parents=True, exist_ok=True)

    args = [
        "yt-dlp",
        "--no-playlist",
        "--skip-download",
        "--sub-langs",
        lang,
        "--sub-format",
        "vtt",
        "--paths",
        str(directory),
    ]
    if kind == "official":
        args.append("--write-subs")
    elif kind == "auto":
        args.append("--write-auto-subs")
    else:
        raise UserFacingError(f"internal: unknown subtitle kind: {kind}")
    args.append(url)

    result = run_command(args)
    if result.returncode != 0:
        return 2, None

    candidates = sorted(path for path in directory.rglob("*.vtt") if path.is_file())
    if not candidates:
        return 1, None

    return 0, pick_best_vtt(lang, candidates)


def clean_vtt_lines(vtt_path: Path) -> list[str]:
    out_lines: list[str] = []
    in_note = False
    in_block = False
    in_header = False
    prev_blank = False
    prev_line = ""

    with vtt_path.open("r", encoding="utf-8-sig", errors="replace") as handle:
        for raw_line in handle:
            line = raw_line.rstrip("\n").rstrip("\r")

            if line.startswith("WEBVTT"):
                in_header = True
                continue
            if in_header:
                if line == "":
                    in_header = False
                continue

            if line == "NOTE" or line.startswith("NOTE "):
                in_note = True
                continue
            if in_note:
                if line == "":
                    in_note = False
                continue

            if line in {"STYLE", "REGION"}:
                in_block = True
                continue
            if in_block:
                if line == "":
                    in_block = False
                continue

            if "-->" in line:
                continue

            line = TAG_RE.sub("", line)
            line = SPACE_RE.sub(" ", line).strip()

            if line == "":
                if not prev_blank:
                    out_lines.append("")
                    prev_blank = True
                continue

            if line == prev_line:
                continue

            out_lines.append(line)
            prev_line = line
            prev_blank = False

    return out_lines


def write_markdown(
    vtt_path: Path,
    md_path: Path,
    title: str,
    url: str,
    lang: str,
    captions_kind: str,
) -> None:
    lines = clean_vtt_lines(vtt_path)
    md_path.parent.mkdir(parents=True, exist_ok=True)

    with md_path.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(f"# {title}\n\n")
        handle.write(f"Source: {url}\n\n")
        handle.write(f"Language: {lang}\n")
        handle.write(f"Captions: {captions_kind}\n\n")
        handle.write("---\n\n")
        if lines:
            handle.write("\n".join(lines))
            handle.write("\n")


def suffix_for_collision(base: str, out_dir: Path, want_subtitle: bool, overwrite: bool) -> str:
    final_base = base
    i = 1
    while not overwrite:
        md_candidate = out_dir / f"{final_base}.md"
        vtt_candidate = out_dir / f"{final_base}.vtt"
        if md_candidate.exists() or (want_subtitle and vtt_candidate.exists()):
            i += 1
            final_base = f"{base}-{i}"
            continue
        break
    return final_base


def availability_message(official_langs: Sequence[str], auto_langs: Sequence[str]) -> str:
    official_text = ", ".join(official_langs) if official_langs else "(none)"
    auto_text = ", ".join(auto_langs) if auto_langs else "(none)"
    return f"official={official_text}; auto={auto_text}"


def positive_int(value: str) -> int:
    try:
        parsed = int(value)
    except ValueError as exc:
        raise argparse.ArgumentTypeError("--max-len must be a positive integer") from exc
    if parsed <= 0:
        raise argparse.ArgumentTypeError("--max-len must be a positive integer")
    return parsed


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog=PROG,
        description="Download YouTube transcripts and write a cleaned Markdown file.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "Output:\n"
            "  Writes: <kebab-title>--<video-id>.md\n"
            "  With --subtitle, also writes: <kebab-title>--<video-id>.vtt\n\n"
            "Notes:\n"
            "  - Requires yt-dlp in PATH.\n"
            "  - Fixed priority order: en official -> en auto -> fr official -> fr auto -> any official -> any auto.\n"
            "  - --lang is accepted for compatibility but ignored.\n"
            "  - Prints the written .md path to stdout (logs go to stderr)."
        ),
        allow_abbrev=False,
    )
    parser.add_argument("-l", "--lang", default="en", help="Deprecated; ignored (fixed priority order is always used)")
    parser.add_argument("-o", "--out-dir", default=".", help="Output directory (default: current directory)")
    parser.add_argument("--max-len", default=80, type=positive_int, help="Max kebab-title length (default: 80)")
    parser.add_argument("--subtitle", action="store_true", help="Also save the raw .vtt subtitle file")
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite existing output files (default: auto-suffix)",
    )
    parser.add_argument("url", help="YouTube video URL")
    return parser


def install_signal_handlers() -> None:
    def handle_interrupt(_: int, __: Any) -> None:
        log("interrupted")
        raise SystemExit(130)

    def handle_hangup(_: int, __: Any) -> None:
        log("hangup")
        raise SystemExit(129)

    def handle_terminate(_: int, __: Any) -> None:
        log("terminated")
        raise SystemExit(143)

    signal.signal(signal.SIGINT, handle_interrupt)
    signal.signal(signal.SIGHUP, handle_hangup)
    signal.signal(signal.SIGTERM, handle_terminate)


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    try:
        require_binary("yt-dlp")
    except UserFacingError as exc:
        log(str(exc))
        return 127

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    install_signal_handlers()

    info = fetch_video_info(args.url)

    video_id = str(info.get("id", "")).strip()
    title = str(info.get("title", "")).strip()
    if not video_id or not title:
        raise UserFacingError("failed to parse video metadata (is the URL valid?)")

    slug = slugify(title, args.max_len)
    base = f"{slug}--{video_id}"
    final_base = suffix_for_collision(base, out_dir, args.subtitle, args.overwrite)

    md_out = out_dir / f"{final_base}.md"
    vtt_out = out_dir / f"{final_base}.vtt"

    attempts, official_langs, auto_langs, step_logs = build_caption_attempts(info)
    if args.lang != "en":
        log(f"warning: ignoring --lang='{args.lang}'; using fixed priority order")

    available = availability_message(official_langs, auto_langs)
    log(f"available subtitles: {available}")
    for line in step_logs:
        log(f"selection {line}")
    if not attempts:
        raise UserFacingError(f"no subtitles found ({available})")

    log(f"starting subtitle download attempts ({len(attempts)} candidate language tags)")

    chosen_kind: Optional[str] = None
    chosen_lang: Optional[str] = None
    vtt_path: Optional[Path] = None
    attempted_labels: list[str] = []

    with tempfile.TemporaryDirectory(prefix="yt-transcript-") as temp_root:
        temp_root_path = Path(temp_root)
        for index, (kind, lang_tag) in enumerate(attempts, start=1):
            attempt_label = f"{kind}:{lang_tag}"
            attempted_labels.append(attempt_label)
            log(f"attempt {index}/{len(attempts)}: trying {attempt_label}")
            target_dir = temp_root_path / f"{kind}-{index}"
            rc, maybe_vtt_path = download_subtitles(kind, args.url, lang_tag, target_dir)
            if rc == 0 and maybe_vtt_path is not None:
                chosen_kind = kind
                chosen_lang = lang_tag
                vtt_path = maybe_vtt_path
                log(f"selected subtitles: {attempt_label}")
                break
            if rc == 2:
                raise UserFacingError(f"yt-dlp failed while fetching subtitles for {attempt_label}")
            log(f"attempt {index}/{len(attempts)} produced no subtitle file for {attempt_label}")

        if vtt_path is None or chosen_kind is None or chosen_lang is None:
            attempted = ", ".join(attempted_labels) if attempted_labels else "(none)"
            raise UserFacingError(f"no subtitles found after trying [{attempted}] ({available})")

        if args.subtitle:
            shutil.copy2(vtt_path, vtt_out)
            log(f"wrote subtitle: {vtt_out} ({chosen_kind}:{chosen_lang})")

        write_markdown(vtt_path, md_out, title, args.url, chosen_lang, chosen_kind)

    log(f"wrote transcript: {md_out}")
    print(str(md_out))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except UserFacingError as exc:
        log(f"error: {exc}")
        raise SystemExit(1) from exc
