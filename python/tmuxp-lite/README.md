# tmuxp-lite

`tmuxp-lite` saves and restores simple tmux sessions from one YAML file.

The first version intentionally keeps the model small:

- all saved sessions live in one YAML file
- `capture` replaces that file with a snapshot of the current tmux server
- `restore` skips sessions that already exist unless `--kill-existing` is used
- windows are restored in YAML order
- pane layouts are restored only as simple horizontal or vertical splits

## Usage

```sh
tmuxp-lite capture
tmuxp-lite list
tmuxp-lite edit
tmuxp-lite restore
tmuxp-lite restore utility-belt --dry-run
```

The default config path is:

```text
$XDG_CONFIG_HOME/tmuxp-lite/sessions.yaml
```

or:

```text
~/.config/tmuxp-lite/sessions.yaml
```
