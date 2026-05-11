#!/usr/bin/env python3

from __future__ import annotations

import argparse
import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Sequence

DEFAULT_IMAGE = "utility-belt/pi-docker:0.73.0"
DEFAULT_PI_VERSION = "0.73.0"
DEFAULT_PI_PACKAGES = ("@ollama/pi-web-search",)
DEFAULT_AGENT_STUFF_DIR = Path("/Users/jfokkan/Developer/jonfk_code/agent-stuff")
DEFAULT_HOME_FILES = (
    ".gitconfig",
    ".gitconfig.common",
    ".gitconfig.jonfk",
    ".gitconfig.work",
    ".gitignore.global",
    ".gitignore.work.global",
)
ENV_ALLOWLIST = (
    "AI_GATEWAY_API_KEY",
    "ANTHROPIC_API_KEY",
    "AZURE_OPENAI_API_KEY",
    "CEREBRAS_API_KEY",
    "GEMINI_API_KEY",
    "GITHUB_TOKEN",
    "GROQ_API_KEY",
    "KIMI_API_KEY",
    "MINIMAX_API_KEY",
    "MINIMAX_CN_API_KEY",
    "MISTRAL_API_KEY",
    "OPENCODE_API_KEY",
    "OPENAI_API_KEY",
    "OPENROUTER_API_KEY",
    "PI_CACHE_RETENTION",
    "PI_CODING_AGENT_DIR",
    "PI_PACKAGE_DIR",
    "PI_SKIP_VERSION_CHECK",
    "SSH_AUTH_SOCK",
    "XAI_API_KEY",
    "ZAI_API_KEY",
)


def die(message: str) -> int:
    print(f"pi-docker: {message}", file=sys.stderr)
    return 1


def dockerfile_default_path() -> Path:
    return Path(__file__).resolve().parent / "Dockerfile"


def run_command(args: Sequence[str]) -> int:
    try:
        completed = subprocess.run(args, check=False)
    except FileNotFoundError:
        return die(f"command not found: {args[0]}")
    except KeyboardInterrupt:
        print("", file=sys.stderr)
        return 130
    return completed.returncode


def strip_remainder_separator(args: Sequence[str]) -> list[str]:
    values = list(args)
    if values and values[0] == "--":
        return values[1:]
    return values


def image_exists(docker: str, image: str) -> bool:
    try:
        completed = subprocess.run(
            [docker, "image", "inspect", image],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
    except FileNotFoundError:
        return False
    return completed.returncode == 0


def existing_path(path: Path) -> Path | None:
    return path if path.exists() or path.is_symlink() else None


def add_mount(args: list[str], source: Path, target: Path | None = None, readonly: bool = False) -> None:
    target = source if target is None else target
    option = f"type=bind,source={source},target={target}"
    if readonly:
        option += ",readonly"
    args.extend(["--mount", option])


def add_existing_mount(
    args: list[str],
    source: Path,
    target: Path | None = None,
    readonly: bool = False,
) -> None:
    resolved = existing_path(source)
    if resolved is not None:
        add_mount(args, resolved, target, readonly)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="pi-docker",
        description="Run Pi inside Docker with host Pi state and the current project mounted.",
    )
    parser.add_argument("--image", default=os.environ.get("PI_DOCKER_IMAGE", DEFAULT_IMAGE))
    parser.add_argument("--docker", default=os.environ.get("PI_DOCKER_DOCKER", "docker"))
    parser.add_argument("--home", type=Path, default=Path.home(), help=argparse.SUPPRESS)

    subparsers = parser.add_subparsers(dest="command")

    build = subparsers.add_parser("build", help="Build the Pi Docker image.")
    build.add_argument("--image", default=os.environ.get("PI_DOCKER_IMAGE", DEFAULT_IMAGE))
    build.add_argument("--pi-version", default=os.environ.get("PI_DOCKER_PI_VERSION", DEFAULT_PI_VERSION))
    build.add_argument(
        "--pi-package",
        action="append",
        default=[],
        help="Npm Pi package to bake into the image. Can be passed multiple times.",
    )
    build.add_argument(
        "--no-default-pi-packages",
        action="store_true",
        help="Do not bake the default Pi packages into the image.",
    )
    build.add_argument("--dockerfile", type=Path, default=dockerfile_default_path())
    build.add_argument("docker_build_args", nargs=argparse.REMAINDER)

    shell = subparsers.add_parser("shell", help="Open a shell in the Pi Docker environment.")
    add_run_options(shell)
    shell.add_argument("shell_args", nargs=argparse.REMAINDER)

    run = subparsers.add_parser("run", help="Run Pi in Docker. This is the default command.")
    add_run_options(run)
    run.add_argument("pi_args", nargs=argparse.REMAINDER)

    return parser


def add_run_options(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--image", default=os.environ.get("PI_DOCKER_IMAGE", DEFAULT_IMAGE))
    parser.add_argument("--mount-pi-root", action="store_true", help="Mount ~/.pi instead of ~/.pi/agent.")
    parser.add_argument(
        "--mount-home-readonly",
        action="store_true",
        help="Mount the host home directory read-only.",
    )
    parser.add_argument("--no-dotfiles", action="store_true", help="Do not mount ~/dotfiles read-only.")
    parser.add_argument("--no-git-config", action="store_true", help="Do not mount common ~/.gitconfig* files.")
    parser.add_argument("--no-agents", action="store_true", help="Do not mount ~/.agents or project .agents read-only.")
    parser.add_argument("--no-ollama-bridge", action="store_true", help="Disable the 127.0.0.1:11434 Ollama bridge.")
    parser.add_argument("--ssh-agent", action="store_true", help="Forward SSH_AUTH_SOCK into the container.")
    parser.add_argument("--docker-arg", action="append", default=[], help="Extra raw argument passed to docker run.")


def build_image(args: argparse.Namespace) -> int:
    dockerfile = args.dockerfile.resolve()
    if not dockerfile.exists():
        return die(f"Dockerfile not found: {dockerfile}")

    packages = []
    env_packages = os.environ.get("PI_DOCKER_PI_PACKAGES", "").split()
    if not args.no_default_pi_packages:
        packages.extend(DEFAULT_PI_PACKAGES)
    packages.extend(env_packages)
    packages.extend(args.pi_package)

    command = [
        args.docker,
        "build",
        "--build-arg",
        f"PI_VERSION={args.pi_version}",
        "--build-arg",
        f"PI_PACKAGES={' '.join(packages)}",
        "-t",
        args.image,
        "-f",
        str(dockerfile),
        str(dockerfile.parent),
        *args.docker_build_args,
    ]
    return run_command(command)


def run_pi(args: argparse.Namespace, pi_args: Sequence[str], shell: bool = False) -> int:
    home = args.home.resolve()
    cwd = Path.cwd().resolve()
    pi_agent_dir = home / ".pi" / "agent"
    npm_global_dir = "/tmp/pi-docker-npm-global"

    if not image_exists(args.docker, args.image):
        return die(f"image {args.image!r} not found. Run: pi-docker build")

    command = [
        args.docker,
        "run",
        "--rm",
        "-it",
        "--user",
        f"{os.getuid()}:{os.getgid()}",
        "--workdir",
        str(cwd),
        "--hostname",
        "pi-docker",
        "--env",
        f"HOME={home}",
        "--env",
        f"USER={os.environ.get('USER', 'piuser')}",
        "--env",
        "PI_DOCKER=1",
        "--env",
        f"PI_DOCKER_IMAGE={args.image}",
        "--env",
        "VISUAL=vim",
        "--env",
        "EDITOR=vim",
        "--env",
        "NPM_CONFIG_CACHE=/tmp/pi-docker-npm-cache",
        "--env",
        f"NPM_CONFIG_PREFIX={npm_global_dir}",
        "--env",
        f"PATH={npm_global_dir}/bin:/opt/pi-docker/npm-global/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        "--env",
        "XDG_CACHE_HOME=/tmp/pi-docker-cache",
        "--env",
        "PI_DOCKER_OLLAMA_BRIDGE=0" if args.no_ollama_bridge else "PI_DOCKER_OLLAMA_BRIDGE=1",
        "--add-host",
        "host.docker.internal:host-gateway",
        "--cap-drop",
        "ALL",
        "--security-opt",
        "no-new-privileges",
    ]

    if args.mount_home_readonly:
        add_mount(command, home, readonly=True)
    add_mount(command, cwd)

    if args.mount_pi_root:
        add_existing_mount(command, home / ".pi", readonly=False)
    else:
        add_existing_mount(command, pi_agent_dir, readonly=False)

    if not args.no_dotfiles:
        add_existing_mount(command, home / "dotfiles", readonly=True)

    if not args.no_git_config:
        for name in DEFAULT_HOME_FILES:
            add_existing_mount(command, home / name, readonly=True)

    if not args.no_agents:
        add_existing_mount(command, home / ".agents", readonly=True)
        add_existing_mount(command, cwd / ".agents", readonly=True)
        add_existing_mount(command, DEFAULT_AGENT_STUFF_DIR)

    for env_name in ENV_ALLOWLIST:
        if env_name == "SSH_AUTH_SOCK" and not args.ssh_agent:
            continue
        value = os.environ.get(env_name)
        if value:
            command.extend(["--env", env_name])

    if args.ssh_agent:
        ssh_sock = os.environ.get("SSH_AUTH_SOCK")
        if not ssh_sock:
            return die("--ssh-agent was passed, but SSH_AUTH_SOCK is not set")
        add_existing_mount(command, Path(ssh_sock))

    if platform.system() == "Linux":
        command.extend(["--env", f"HOST_UID={os.getuid()}", "--env", f"HOST_GID={os.getgid()}"])

    command.extend(args.docker_arg)
    command.append(args.image)
    if shell:
        command.append("shell")
        command.extend(strip_remainder_separator(pi_args))
    else:
        command.extend(strip_remainder_separator(pi_args))

    return run_command(command)


def normalize_default_command(argv: Sequence[str]) -> list[str]:
    if not argv:
        return ["run"]
    first = argv[0]
    if first in {"build", "run", "shell", "-h", "--help"} or first.startswith("--"):
        return list(argv)
    return ["run", *argv]


def main(argv: Sequence[str] | None = None) -> int:
    raw_args = list(sys.argv[1:] if argv is None else argv)
    parser = build_parser()
    parsed = parser.parse_args(normalize_default_command(raw_args))

    if shutil.which(parsed.docker) is None:
        return die(f"Docker command not found: {parsed.docker}")

    if parsed.command == "build":
        return build_image(parsed)
    if parsed.command == "shell":
        return run_pi(parsed, parsed.shell_args, shell=True)
    if parsed.command == "run":
        return run_pi(parsed, parsed.pi_args)

    parser.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
