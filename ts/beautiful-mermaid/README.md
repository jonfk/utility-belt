# beautiful-mermaid CLI (temporary)

This is a small CLI wrapper around the `beautiful-mermaid` library for rendering Mermaid diagrams as SVG or ASCII.

Upstream does not currently provide an official CLI. Monitor the README at https://github.com/lukilabs/beautiful-mermaid and deprecate this tool once a supported CLI is published there.

## Setup

```bash
bun install
```

## Build

```bash
bun run build
# Output: dist/beautiful-mermaid
```

## Usage

```bash
beautiful-mermaid [options] [file]
```

Examples:

```bash
# SVG (default). Output file derived from input name.
beautiful-mermaid diagram.mmd

# SVG from stdin requires an explicit output file.
printf "graph TD\nA-->B\n" | beautiful-mermaid --output diagram.svg

# ASCII to stdout.
beautiful-mermaid --ascii diagram.mmd

# ASCII to file.
beautiful-mermaid --ascii --output diagram.txt diagram.mmd

# SVG with a built-in theme.
beautiful-mermaid --theme tokyo-night diagram.mmd

# List available themes.
beautiful-mermaid --themes
```

## Options

- `-a, --ascii` Output ASCII instead of SVG.
- `--svg` Force SVG output (default).
- `-o, --output <file>` Write output to a file.
- `-t, --theme <name>` Apply a built-in theme for SVG output.
- `--themes` List available themes and exit.

## Notes

- SVG is the default output format.
- When reading from a file, SVG output is written to `<input>.svg` unless `--output` is provided.
- When reading from stdin, SVG output requires `--output`.
- ASCII output defaults to stdout unless `--output` is provided.
