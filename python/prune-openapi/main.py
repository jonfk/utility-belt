"""CLI for pruning OpenAPI specs by selected operations."""

from __future__ import annotations

import json
import logging
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path
from tempfile import NamedTemporaryFile
from typing import Dict, Iterable, List, Optional, Sequence

import typer


app = typer.Typer(
    add_completion=False,
    context_settings={"help_option_names": ["-h", "--help"]},
)

HTTP_METHODS: Sequence[str] = (
    "get",
    "post",
    "put",
    "patch",
    "delete",
    "options",
    "head",
    "trace",
)

DEFAULT_OUTPUT_NAME = "pruned-openapi.yaml"


@dataclass
class OperationEntry:
    path: str
    method: str
    operation_id: Optional[str]
    summary: str
    display: str


def configure_logging(verbose: bool) -> None:
    level = logging.DEBUG if verbose else logging.INFO
    logging.basicConfig(level=level, format="%(levelname)s: %(message)s")


def ensure_input_file(path: Path) -> Path:
    if not path.exists():
        raise typer.BadParameter(f"Input file not found: {path}")
    if not path.is_file():
        raise typer.BadParameter(f"Input path is not a file: {path}")
    if path.suffix.lower() != ".json":
        message = (
            "Only JSON OpenAPI specifications are supported. "
            "YAML input is not supported."
        )
        raise typer.BadParameter(message)
    return path


def load_spec(path: Path) -> Dict[str, object]:
    try:
        with path.open("r", encoding="utf-8") as fh:
            data = json.load(fh)
    except json.JSONDecodeError as exc:
        typer.echo(f"Failed to parse JSON: {exc}", err=True)
        raise typer.Exit(code=1) from exc
    if not isinstance(data, dict):
        typer.echo("The OpenAPI document must be a JSON object.", err=True)
        raise typer.Exit(code=1)
    return data


def validate_openapi_version(spec: Dict[str, object]) -> None:
    version = spec.get("openapi")
    if not isinstance(version, str) or not version.startswith("3."):
        typer.echo(
            "This tool only supports OpenAPI 3.x specifications.",
            err=True,
        )
        raise typer.Exit(code=1)


def collect_operations(spec: Dict[str, object]) -> List[OperationEntry]:
    paths = spec.get("paths")
    if not isinstance(paths, dict):
        typer.echo("No paths section found in the OpenAPI specification.", err=True)
        raise typer.Exit(code=1)

    entries: List[OperationEntry] = []
    for path, item in paths.items():
        if not isinstance(item, dict):
            continue
        for method in HTTP_METHODS:
            raw_operation = item.get(method)
            if not isinstance(raw_operation, dict):
                continue
            operation_id = raw_operation.get("operationId")
            summary = raw_operation.get("summary")
            description = raw_operation.get("description")
            summary_text = _clean_summary(summary or description)
            display = _format_display(method, path, operation_id, summary_text)
            entries.append(
                OperationEntry(
                    path=path,
                    method=method.upper(),
                    operation_id=operation_id if isinstance(operation_id, str) else None,
                    summary=summary_text,
                    display=display,
                )
            )
    if not entries:
        typer.echo("No operations found in the OpenAPI specification.", err=True)
        raise typer.Exit(code=1)
    return entries


def _clean_summary(value: Optional[str]) -> str:
    if not value:
        return "[no summary]"
    return " ".join(value.split())


def _format_display(method: str, path: str, operation_id: Optional[str], summary: str) -> str:
    op_segment = operation_id if operation_id else "[missing operationId]"
    return f"{method.upper()} {path} | {op_segment} - {summary}"


def select_operations_interactively(entries: List[OperationEntry]) -> List[OperationEntry]:
    selection_input = "\n".join(entry.display for entry in entries)
    result = subprocess.run(
        [
            "fzf",
            "--multi",
            "--prompt",
            "Select operations> ",
            "--header",
            "Press TAB to toggle selections; ENTER to confirm.",
        ],
        input=selection_input,
        text=True,
        capture_output=True,
    )

    if result.returncode != 0:
        if result.returncode == 130:
            typer.echo("Selection cancelled.", err=True)
        else:
            typer.echo(result.stderr or "fzf selection failed.", err=True)
        raise typer.Exit(code=1)

    selected_lines = [line for line in result.stdout.splitlines() if line.strip()]
    if not selected_lines:
        typer.echo("No operations selected.", err=True)
        raise typer.Exit(code=1)

    display_map = {entry.display: entry for entry in entries}
    chosen: List[OperationEntry] = []
    for line in selected_lines:
        entry = display_map.get(line)
        if not entry:
            typer.echo(f"Unknown selection returned by fzf: {line}", err=True)
            raise typer.Exit(code=1)
        chosen.append(entry)
    return chosen


def select_operations_by_id(
    entries: List[OperationEntry],
    requested_ids: Iterable[str],
) -> List[OperationEntry]:
    id_index: Dict[str, OperationEntry] = {
        entry.operation_id: entry for entry in entries if entry.operation_id
    }

    missing: List[str] = []
    seen: Dict[str, bool] = {}
    chosen: List[OperationEntry] = []
    for operation_id in requested_ids:
        if operation_id in seen:
            continue
        seen[operation_id] = True
        entry = id_index.get(operation_id)
        if not entry:
            missing.append(operation_id)
            continue
        chosen.append(entry)

    if missing:
        missing_csv = ", ".join(missing)
        typer.echo(f"Unknown operationId(s): {missing_csv}", err=True)
        raise typer.Exit(code=1)

    if not chosen:
        typer.echo("No valid operations resolved from provided operationIds.", err=True)
        raise typer.Exit(code=1)
    return chosen


def ensure_operation_ids(entries: List[OperationEntry]) -> None:
    missing = [f"{entry.method} {entry.path}" for entry in entries if not entry.operation_id]
    if missing:
        details = "; ".join(missing)
        typer.echo(
            f"Selected operations missing operationId: {details}",
            err=True,
        )
        raise typer.Exit(code=1)


def run_openapi_extract(
    input_path: Path,
    operations: List[OperationEntry],
    verbose: bool,
) -> Path:
    with NamedTemporaryFile("w", delete=False, suffix=".json") as tmp:
        temp_output = Path(tmp.name)

    command = ["pnpm", "dlx", "openapi-extract"]
    for entry in operations:
        # operation_id presence enforced earlier
        command.extend(["-o", entry.operation_id])
    command.extend(["--", str(input_path), str(temp_output)])

    run_command(command, verbose)
    return temp_output


def convert_output(
    temp_json: Path,
    output_path: Path,
    verbose: bool,
) -> None:
    suffix = output_path.suffix.lower()
    output_path.parent.mkdir(parents=True, exist_ok=True)
    if suffix in {".yaml", ".yml"}:
        command = [
            "pnpm",
            "--package=@redocly/cli",
            "dlx",
            "redocly",
            "bundle",
            str(temp_json),
            "--remove-unused-components",
            "--ext",
            "yaml",
            "-o",
            str(output_path),
        ]
        run_command(command, verbose)
    else:
        temp_json.replace(output_path)


def run_command(command: List[str], verbose: bool) -> None:
    log_line = " ".join(command)
    logging.debug("Running command: %s", log_line)
    try:
        subprocess.run(
            command,
            check=True,
            stdout=None if verbose else subprocess.PIPE,
            stderr=None if verbose else subprocess.PIPE,
            text=True,
        )
    except subprocess.CalledProcessError as exc:
        if exc.stdout:
            typer.echo(exc.stdout, err=True)
        if exc.stderr:
            typer.echo(exc.stderr, err=True)
        typer.echo(f"Command failed: {log_line}", err=True)
        raise typer.Exit(code=exc.returncode)


@app.command()
def main(
    input_path: Path = typer.Argument(..., help="Path to an OpenAPI 3 JSON file."),
    output: Optional[Path] = typer.Option(
        None,
        "--output",
        "-o",
        help="Destination for the pruned spec (defaults to pruned-openapi.yaml).",
    ),
    operations: Optional[List[str]] = typer.Option(
        None,
        "--operations",
        help="OperationId to prune (repeatable). Skips fzf when provided.",
    ),
    verbose: bool = typer.Option(
        False,
        "--verbose",
        "-v",
        help="Enable verbose logging.",
    ),
) -> None:
    """Prune an OpenAPI specification down to selected operations.

    The CLI shells out to `fzf` for interactive selection when no `--operations`
    are provided, so ensure the binary is available in `PATH`. The output format
    is inferred from the destination file extension: `.yaml` / `.yml` triggers a
    conversion to YAML, while other extensions keep the JSON.
    """

    configure_logging(verbose)

    resolved_input = ensure_input_file(input_path.resolve())
    output_path = output.resolve() if output else Path(DEFAULT_OUTPUT_NAME).resolve()

    # Tool availability checks
    for binary in ("pnpm", "fzf"):
        verify_binary(binary)

    spec = load_spec(resolved_input)
    validate_openapi_version(spec)
    entries = collect_operations(spec)

    selected: List[OperationEntry]
    if operations:
        selected = select_operations_by_id(entries, operations)
    else:
        selected = select_operations_interactively(entries)

    ensure_operation_ids(selected)

    temp_json = run_openapi_extract(resolved_input, selected, verbose)
    try:
        convert_output(temp_json, output_path, verbose)
    finally:
        if temp_json.exists():
            temp_json.unlink()

    typer.echo("Selected operations:")
    for entry in selected:
        typer.echo(entry.operation_id)


def verify_binary(name: str) -> None:
    if shutil.which(name) is None:
        typer.echo(f"Required binary not found in PATH: {name}", err=True)
        raise typer.Exit(code=1)


if __name__ == "__main__":
    app()
