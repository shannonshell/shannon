# Getting Started

## Installation

Install from [crates.io](https://crates.io/crates/shannonshell):

```sh
cargo install shannonshell
```

Or build from source:

```sh
git clone --recursive https://github.com/shannonshell/shannon.git
cd shannon/shannon
cargo build --release
```

Note: `--recursive` is needed to fetch the nushell, brush, and reedline
submodules.

## First Run

```sh
shannon
```

You'll see:

```
Welcome to Shannon, based on the Nu language, where all data is structured!
Version: 0.3.2
Startup Time: 125ms

[nu] ~/projects >
```

Shannon starts in nushell mode. All nushell commands work.

## Switching Modes

Press **Shift+Tab** to cycle between modes:

```
[nu] ~/projects > ls | where size > 1mb    ← nushell
[brush] ~/projects > grep -r TODO src/     ← bash
[ai] ~/projects > how do I find large files? ← AI chat
```

Your environment variables and working directory carry over when you switch.

## Running Commands

In **nu mode**, use nushell syntax:

```
[nu] ~/project > ls | sort-by modified
[nu] ~/project > $env.HOME
```

In **brush mode**, use bash syntax:

```
[brush] ~/project > echo hello && echo world
[brush] ~/project > export FOO=bar
```

In **ai mode**, type plain English:

```
[ai] ~/project > how do I compress a folder?
```

## Setting Up Your Environment

Create `~/.config/shannon/env.sh` for bash-style environment setup:

```bash
# ~/.config/shannon/env.sh
eval "$(/opt/homebrew/bin/brew shellenv)"
export PATH="$PATH:$HOME/.cargo/bin"
export ANTHROPIC_API_KEY="sk-ant-..."
```

This runs via brush at startup, before nushell's `env.nu` and `config.nu`.
Follow any tutorial that says "add this to your .bashrc" — it works in
`env.sh`.

## Exiting

- **Ctrl+D** — exit shannon
- Type `exit` — also exits (works in all modes)
