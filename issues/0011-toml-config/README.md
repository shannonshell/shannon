+++
status = "closed"
opened = "2026-03-22"
closed = "2026-03-22"
+++

# Issue 11: TOML configuration and custom shell support

## Goal

Add a `config.toml` for shannon-specific settings, support custom shells via
configuration (no code changes needed), and rename `config.sh` to `env.sh` to
clarify its purpose.

## Background

Shannon currently has two configuration mechanisms:

1. **`config.sh`** — a bash script that sets environment variables (PATH, API
   keys, etc.). This is for importing the user's environment, not for
   configuring shannon itself.
2. **`SHANNON_DEFAULT_SHELL` env var** — set in `config.sh` to pick the default
   shell. This works but conflates shannon settings with environment setup.

Additionally, shell support is hardcoded in Rust — adding a new shell requires
modifying `shell.rs`, `executor.rs`, `highlighter.rs`, `main.rs`, and
`prompt.rs`. This means users can't add shells (like zsh, elvish, tcsh) without
recompiling.

### Design

#### No files are generated

Shannon does NOT write config files to disk. Defaults live in the binary. If no
`config.toml` exists, shannon works exactly as it does today. The user only
creates files when they want to change something.

This avoids the upgrade problem: when shannon ships a better default wrapper,
users who haven't customized get the improvement automatically. Users who have
customized are in control of their own config.

Documentation shows what the defaults look like so users can copy and modify.

#### Two config files, two purposes

| File                       | Purpose                             | Format      |
| -------------------------- | ----------------------------------- | ----------- |
| `env.sh` (was `config.sh`) | Environment setup (PATH, API keys)  | Bash script |
| `config.toml`              | Shannon settings (shells, defaults) | TOML        |

`env.sh` runs once at startup to set up the environment. `config.toml` is read
once at startup to configure shannon itself. They serve completely different
purposes and should not be conflated.

Shannon checks `env.sh` first, falls back to `config.sh` for backward
compatibility.

#### config.toml structure

The config is **partial by default**. The user only specifies what they want to
change. Everything else uses built-in defaults.

Minimal example — just change the default shell:

```toml
default_shell = "nu"
```

Full example — add zsh and customize:

```toml
default_shell = "nu"

[shells.zsh]
binary = "zsh"
init = "shells/zsh/init.zsh"
highlighter = "bash"
parser = "env"
wrapper = """
{{init}}
{{command}}
__shannon_ec=$?
env > '{{temp_path}}'
echo "__SHANNON_CWD=$(pwd)" >> '{{temp_path}}'
echo "__SHANNON_EXIT=$__shannon_ec" >> '{{temp_path}}'
exit $__shannon_ec
"""
```

This adds zsh to the rotation alongside the built-in shells. To override a
built-in shell's wrapper, redefine its section (e.g. `[shells.bash]`).

#### Wrapper templates

Each shell's wrapper is a string with three placeholders:

| Placeholder     | Replaced with                          |
| --------------- | -------------------------------------- |
| `{{command}}`   | The user's command                     |
| `{{temp_path}}` | Path to the temp file for env capture  |
| `{{init}}`      | Contents of the init script (or empty) |

The wrapper is responsible for running the command, capturing env/cwd/exit code
to the temp file. The user can see exactly what runs.

Built-in defaults match what's currently hardcoded in `executor.rs`.

#### Per-shell init scripts

Optional scripts that run before each command inside the wrapper. External files
with the correct extension so editors highlight them:

```
~/.config/shannon/shells/bash/init.sh
~/.config/shannon/shells/nu/init.nu
~/.config/shannon/shells/fish/init.fish
~/.config/shannon/shells/zsh/init.zsh
```

The `init` field is a path relative to the config directory. If the file doesn't
exist or the field is omitted, `{{init}}` expands to nothing. No error.

Use cases:

- Load nushell standard library: `use std *`
- Set bash options: `shopt -s globstar`
- Define fish abbreviations
- Set up shell-specific aliases

#### Env parsers

The `parser` field tells shannon how to read the temp file:

| Parser    | Format                                                         | Used by                    |
| --------- | -------------------------------------------------------------- | -------------------------- |
| `bash`    | `declare -x KEY="VALUE"` lines + `__SHANNON_*` markers         | bash                       |
| `nushell` | JSON object from `$env \| to json`                             | nushell                    |
| `env`     | `KEY=VALUE` lines (from `env` command) + `__SHANNON_*` markers | fish, zsh, any POSIX shell |

The `env` parser is the generic default. Most new shells will use it.

#### Syntax highlighting

Tree-sitter grammars are compile-time dependencies. Shannon ships grammars for
bash, nushell, and fish. The `highlighter` field maps a shell to a built-in
grammar:

```toml
[shells.zsh]
highlighter = "bash"  # use bash grammar for zsh (close enough)
```

Valid values: `bash`, `nushell`, `fish`. If omitted, no highlighting (plain text
input). The shell still works — it just doesn't have colored input.

#### Error handling

| Situation                           | Behavior                             |
| ----------------------------------- | ------------------------------------ |
| No `config.toml`                    | Built-in defaults (current behavior) |
| Bad TOML syntax                     | Error message, exit                  |
| Shell missing `binary`              | Error, skip that shell               |
| Shell missing `wrapper`             | Error, skip that shell               |
| Shell binary not installed          | Silently skip                        |
| Init file missing                   | Silent, `{{init}}` is empty          |
| Init file has errors                | Command fails, user sees the error   |
| Wrapper produces unparseable output | Fall back to previous state          |
| No shells available                 | Error message, exit                  |

Principle: config errors are loud, runtime errors are graceful.

### What TOML replaces

| Before                                       | After                                |
| -------------------------------------------- | ------------------------------------ |
| `SHANNON_DEFAULT_SHELL` env var              | `default_shell` in config.toml       |
| Hardcoded shell list in `main.rs`            | `[shells.*]` tables in config.toml   |
| Hardcoded prompt colors in `prompt.rs`       | Either dropped or configurable       |
| Per-shell wrapper functions in `executor.rs` | Wrapper templates in config.toml     |
| No per-shell init scripts                    | `init` field + external files        |
| `config.sh` name                             | `env.sh` (with `config.sh` fallback) |

### Migration

- `config.sh` continues to work (checked as fallback if `env.sh` doesn't exist)
- `SHANNON_DEFAULT_SHELL` env var continues to work (overridden by config.toml
  if both are set)
- If no config.toml exists, built-in defaults match current behavior exactly

## Experiments

### Experiment 1: Config-driven shells with TOML and wrapper templates

#### Description

Replace the hardcoded shell system with a config-driven one. This is one large
experiment because all the pieces are tightly coupled — you can't usefully have
wrapper templates without the config loader, or the config loader without the
new executor.

#### Changes

**`Cargo.toml`** — add `toml = "0.8"` dependency.

**`src/config.rs`** (new module) — config loading and built-in defaults:

```rust
#[derive(Deserialize, Default)]
pub struct ShannonConfig {
    pub default_shell: Option<String>,
    #[serde(default)]
    pub shells: HashMap<String, ShellConfig>,
}

#[derive(Deserialize)]
pub struct ShellConfig {
    pub binary: String,
    pub wrapper: String,
    pub parser: Option<String>,       // "bash", "nushell", "env" (default: "env")
    pub highlighter: Option<String>,  // "bash", "nushell", "fish", or omitted
    pub init: Option<String>,         // path relative to config dir
}
```

`ShannonConfig::load()`:

1. Check for `config.toml` in config dir. If missing, return empty config.
2. Parse TOML. If bad syntax, print error and exit.
3. Return the parsed config.

`ShannonConfig::shells()` → `Vec<(String, ShellConfig)>`:

1. Start with built-in defaults (bash, nushell, fish) with their current
   wrappers as the default `wrapper` string.
2. Merge user config on top — user-defined shells are added, user-redefined
   built-in shells override the defaults.
3. Return ordered list: `default_shell` first, then the rest in definition
   order.

Built-in defaults are const strings in `config.rs` — the exact wrappers
currently in `executor.rs`, plus parser and highlighter fields.

**`src/executor.rs`** — replace per-shell wrappers with generic execution:

Remove `build_bash_wrapper`, `build_nushell_wrapper`, `build_fish_wrapper`.
Remove `ShellKind` from `execute_command` signature.

New signature:

```rust
pub fn execute_command(
    shell_config: &ShellConfig,
    command: &str,
    state: &ShellState,
) -> io::Result<ShellState>
```

The function:

1. Read the init file if specified (resolve path relative to config dir).
2. Build the wrapper by replacing `{{command}}`, `{{temp_path}}`, `{{init}}` in
   `shell_config.wrapper`.
3. Run `Command::new(&shell_config.binary).args(["-c", &wrapper])...`
4. Parse output using the parser specified by `shell_config.parser`:
   - `"bash"` → `parse_bash_env`
   - `"nushell"` → `parse_nushell_env`
   - `"env"` → `parse_fish_env` (rename to `parse_env`)
   - Default → `parse_env`

Keep the three parser functions — they don't change. Rename `parse_fish_env` to
`parse_env` since it's the generic KEY=VALUE parser.

Also update `run_startup_script` to accept a config dir path instead of using
`ShellKind`.

**`src/shell.rs`** — simplify dramatically:

Remove `ShellKind` enum entirely. Remove `history_file()`. Keep `config_dir()`,
`history_db()`, `ShellState`, and `ShellState::from_current_env()`.

The concept of "which shell" is now a string name + `ShellConfig`, not an enum
variant.

**`src/highlighter.rs`** — make grammar selection string-based:

Change `TreeSitterHighlighter::new(shell: ShellKind)` to
`TreeSitterHighlighter::new(highlighter: Option<&str>)`.

- `Some("bash")` → bash grammar + bash colors
- `Some("nushell")` → nushell grammar + nushell colors
- `Some("fish")` → fish grammar + fish colors
- `None` → no-op highlighter (returns unstyled text)

**`src/prompt.rs`** — use shell name string instead of enum:

`ShannonPrompt` takes `shell_name: String` instead of `ShellKind`. Drop
per-shell colors — use a single prompt color for all shells. The `[shell_name]`
text already identifies which shell is active.

**`src/main.rs`** — rewrite startup to be config-driven:

1. Load `ShannonConfig`.
2. Run `env.sh` (check `env.sh` first, fall back to `config.sh`).
3. Get the ordered shell list from config.
4. Filter to installed shells (`binary --version` check).
5. Build editor using the active shell's config.
6. Main loop passes `&ShellConfig` to `execute_command` instead of `ShellKind`.
7. Shift+Tab cycles through the config-driven shell list.

**`src/lib.rs`** — add `pub mod config;`.

#### What gets removed

- `ShellKind` enum and all match arms across 5 files
- `build_bash_wrapper`, `build_nushell_wrapper`, `build_fish_wrapper`
- `SHANNON_DEFAULT_SHELL` env var handling (replaced by config.toml, but still
  works as fallback)
- Per-shell prompt colors (replaced by single color)

#### Tests

**`src/config.rs`** tests:

- `test_empty_config` — no config.toml, returns built-in defaults
- `test_default_shell` — `default_shell = "nu"` puts nushell first
- `test_custom_shell` — adding zsh via config merges with built-ins
- `test_override_builtin` — redefining `[shells.bash]` overrides default
- `test_bad_toml` — invalid TOML is detected

**`src/executor.rs`** tests:

- Update existing parser tests (unchanged logic)
- `test_template_expansion` — `{{command}}` and `{{temp_path}}` replaced
- `test_template_with_init` — `{{init}}` replaced with file contents
- `test_template_without_init` — `{{init}}` replaced with empty string

**`tests/integration.rs`** — update to use config-driven execution:

- Tests construct `ShellConfig` directly instead of using `ShellKind`
- Same test logic, different API

#### Verification

1. `cargo build` succeeds.
2. `cargo test` passes — all tests green.
3. `cargo run` with NO config.toml — behaves exactly as before (bash, nushell,
   fish rotation).
4. Create minimal `config.toml` with `default_shell = "nu"` — nushell is the
   default.
5. Add `[shells.zsh]` with wrapper — zsh appears in Shift+Tab rotation.
6. Create `~/.config/shannon/shells/nu/init.nu` with `use std *` — nushell
   commands have std library available.
7. Syntax highlighting works for built-in shells, falls back to plain text for
   custom shells.
8. `env.sh` works. `config.sh` fallback works.

**Result:** Pass

All verification steps confirmed. 60 tests pass (44 unit + 16 integration).
The `ShellKind` enum is gone — shells are now string names with `ShellConfig`
structs. Built-in defaults for bash, nushell, and fish match previous behavior
exactly. Custom shells can be added via config.toml.

#### Conclusion

Config-driven shell support is complete. The entire shell system is now driven
by configuration rather than hardcoded enum variants. Wrapper templates use
`{{placeholder}}` syntax. Parsers are selectable by name. Highlighting maps
to built-in grammars by string. The `env.sh` rename with `config.sh` fallback
preserves backward compatibility.

## Conclusion

Issue complete. Shannon now supports TOML configuration and custom shells.

Key files:
- `src/config.rs` — config loading, built-in defaults, wrapper templates
- `src/executor.rs` — generic execution with config-driven wrappers and parsers
- `src/main.rs` — config-driven startup and shell rotation
- `src/highlighter.rs` — string-based grammar selection
- `src/prompt.rs` — string-based shell name, single prompt color
