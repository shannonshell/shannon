# Getting Started

## Installation

```sh
cargo install --git https://github.com/shannonshell/shannon
```

Or build from source:

```sh
git clone https://github.com/shannonshell/shannon.git
cd shannon
cargo build --release
```

## Setting Up Your Environment

**This is the most important step.** Shannon is a new shell — it doesn't
automatically inherit your PATH, homebrew, or other environment setup from
bash/zsh. You need to tell Shannon where your programs are.

The easiest way: create `~/.config/shannon/env.sh` and put your PATH setup
there. This is a bash script that runs at startup.

### Option 1: Copy from your existing shell (recommended)

If you use zsh (macOS default), grab your PATH:

```sh
# Run this in your current shell to see your PATH
echo $PATH
```

Then create env.sh with it:

```bash
# ~/.config/shannon/env.sh
export PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:$HOME/.cargo/bin"
```

### Option 2: Copy your bash/zsh setup

If you have a `.bashrc`, `.bash_profile`, or `.zshrc` with environment setup,
copy the relevant `export` and `eval` lines into env.sh:

```bash
# ~/.config/shannon/env.sh

# Homebrew
eval "$(/opt/homebrew/bin/brew shellenv)"

# Cargo/Rust
export PATH="$PATH:$HOME/.cargo/bin"

# Go
export PATH="$PATH:$HOME/go/bin"

# Node (nvm, fnm, etc.)
eval "$(fnm env)"
```

### Option 3: Use nushell's env.nu

If you're already a nushell user, put your PATH in `~/.config/shannon/env.nu`
using nushell syntax:

```nushell
# ~/.config/shannon/env.nu
$env.PATH = ($env.PATH | prepend "/opt/homebrew/bin")
$env.PATH = ($env.PATH | append ($env.HOME | path join ".cargo/bin"))
```

### Loading order

Shannon loads configuration in this order:

1. `~/.config/shannon/env.sh` — bash environment (via brush)
2. `~/.config/shannon/env.nu` — nushell environment
3. `~/.config/shannon/config.nu` — nushell settings

Any tutorial that says "add this to your `.bashrc`" — put it in `env.sh`.

## First Run

```sh
shannon
```

You'll see:

```
Welcome to Shannon, based on the Nu language, where all data is structured!
Version: 0.3.4
Startup Time: 125ms

[nu] ~/projects >
```

Shannon starts in nushell mode. All nushell commands work.

If commands like `git`, `node`, or `brew` aren't found, your PATH isn't set
up — go back to the environment setup step above.

## Switching Modes

Press **Shift+Tab** to toggle between modes:

```
[nu] ~/projects > ls | where size > 1mb    ← nushell
[bash] ~/projects > grep -r TODO src/     ← bash
```

Your environment variables and working directory carry over when you switch.

## Running Commands

In **nu mode**, use nushell syntax:

```
[nu] ~/project > ls | sort-by modified
[nu] ~/project > $env.HOME
```

In **bash mode**, use bash syntax:

```
[bash] ~/project > echo hello && echo world
[bash] ~/project > export FOO=bar
```

## Exiting

- **Ctrl+D** — exit shannon
- Type `exit` — also exits (works in all modes)
