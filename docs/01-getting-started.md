# Getting Started

## Installation

Shannon requires [Rust](https://www.rust-lang.org/tools/install). Clone the
repo and build:

```sh
git clone https://github.com/yourusername/shannon.git
cd shannon
cargo build --release
```

The binary is at `target/release/shannon`. Copy it to somewhere in your PATH,
or run it directly.

## First Run

```sh
cargo run
```

You'll see a prompt like this:

```
[bash] ~/projects/shannon >
```

This tells you:

- `[bash]` — the active shell is bash
- `~/projects/shannon` — your current working directory
- `>` — the last command succeeded (you'd see `!` if it failed)

## Running Commands

Type any command as you normally would:

```
[bash] ~/projects/shannon > echo "hello from shannon"
hello from shannon
[bash] ~/projects/shannon > ls src/
completer.rs  executor.rs  highlighter.rs  lib.rs  main.rs  prompt.rs  shell.rs
```

Commands run in real shell subprocesses — everything works exactly as it would
in a normal bash or nushell session.

## Switching Shells

Press **Shift+Tab** to switch to the next shell:

```
[bash] ~/projects/shannon > export GREETING="hello"
[bash] ~/projects/shannon > <Shift+Tab>
[nu] ~/projects/shannon > $env.GREETING
hello
```

Your environment variables and working directory carry over when you switch.
See [Shell Switching](features/01-shell-switching.md) for details.

## Tab Completion

Press **Tab** to complete file and directory names:

```
[bash] ~/projects/shannon > cat Car<Tab>
Cargo.lock  Cargo.toml
```

See [Tab Completion](features/05-tab-completion.md) for details.

## Setting Up Your Environment

If shannon is your default shell (e.g. in your terminal emulator config), you
may need to configure PATH and other environment variables. Create a startup
script:

```bash
# ~/.config/shannon/config.sh
eval "$(/opt/homebrew/bin/brew shellenv)"
export PATH="$PATH:$HOME/.cargo/bin"
```

This runs once when shannon starts. See
[Configuration](reference/02-configuration.md) for details.

## Exiting

- **Ctrl+D** — exit shannon
- Type `exit` — also exits shannon
