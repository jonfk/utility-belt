# pi-docker

Run Pi inside Docker while keeping host sessions, project-local `.pi/` settings,
git config, and Ollama access working.

## Install

```sh
just install-pi-docker
```

## Build the image

```sh
pi-docker build
```

Or from the repository root:

```sh
just build-pi-docker
```

By default, `pi-docker build` writes `utility-belt/pi-docker:latest`, and
`pi-docker` runs that tag. To update Pi, rebuild with a different package
version:

```sh
pi-docker build --pi-version 0.74.0
```

The default image also bakes in `@ollama/pi-web-search`, matching the package in
the current Pi config. Add more baked packages with repeated `--pi-package`:

```sh
pi-docker build --pi-package @scope/package-name
```

Set `PI_DOCKER_PI_PACKAGES` to pass a space-separated package list, or use
`--no-default-pi-packages` for an image with no baked Pi packages.

To customize the image, copy the packaged Dockerfile and build with:

```sh
pi-docker build --dockerfile ./Dockerfile
```

## Run Pi

```sh
pi-docker
pi-docker --help
pi-docker -p "summarize this repo"
```

The wrapper mounts:

- the current directory at the same absolute path
- `~/.pi/agent` at the same absolute path
- `~/dotfiles` read-only, when present
- common `.gitconfig*` and global gitignore files read-only, when present
- `~/.agents` and project `.agents` read-only, when present
- `/Users/jfokkan/Developer/jonfk_code/agent-stuff`, when present

It also starts a local container bridge from `127.0.0.1:11434` to
`host.docker.internal:11434`, so existing Pi Ollama configs can keep using
`http://127.0.0.1:11434/v1`.

When running interactively, the container sets the terminal title to
`pi-docker: <cwd>`, sets the Docker hostname to `pi-docker`, and exposes
`PI_DOCKER=1`. It also prints a one-line `[pi-docker]` startup banner before Pi
starts. Disable title updates with `PI_DOCKER_SET_TITLE=0`, and disable the
banner with `PI_DOCKER_BANNER=0`.

Provider API key variables are forwarded by allowlist. Host `VISUAL` and
`EDITOR` are not forwarded because host editor commands often do not exist
inside the container; both default to `vim` in the image.

The host home directory is not mounted by default. Use `--mount-home-readonly`
when you need broad read-only home access. Npm cache writes are redirected to
container-local temp directories. Runtime npm global installs are redirected to
container-local temp directories too; frequently used Pi npm packages should be
baked into the image instead. Pi config, credentials, and sessions still persist
through the writable `~/.pi/agent` mount.

## Shell

```sh
pi-docker shell
```
