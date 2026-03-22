+++
status = "closed"
opened = "2026-03-22"
closed = "2026-03-22"
+++

# Issue 12: Add zsh as built-in default and toggle list

## Goal

Add zsh as a fourth built-in shell and add a `toggle` config option that lets
users control which shells appear in the Shift+Tab rotation. Replace
`default_shell` with `toggle` — the first entry in the list is the default.

## Background

Shannon currently has three built-in shells: bash, nushell, and fish. Zsh is
arguably more common than fish — it's the default shell on macOS. Users
shouldn't need to write a config.toml just to get zsh support.

Additionally, users may not want all installed shells in the rotation. If
someone only uses bash and nushell, cycling through fish and zsh is noise.

### What changes

**Add zsh as a built-in default:**

- Binary: `zsh`
- Wrapper: same generic pattern as fish (uses `env` command for capture)
- Parser: `env`
- Highlighter: `bash` (zsh syntax is close enough)

**Replace `default_shell` with `toggle`:**

The `toggle` list controls both which shells appear and their order. The
first shell in the list is the default. This replaces `default_shell` — one
setting instead of two.

```toml
toggle = ["nu", "bash"]
```

If `toggle` is omitted, all installed built-in + custom shells are available
in default order: bash → nu → fish → zsh.

If a shell in the toggle list isn't installed, it's silently skipped.

### Examples

No config.toml (default behavior):

```
bash → nu → fish → zsh  (all installed shells)
```

Toggle list:

```toml
toggle = ["nu", "bash"]
```

```
nu → bash  (nu is default, only these two in rotation)
```

Toggle with all shells reordered:

```toml
toggle = ["fish", "zsh", "nu", "bash"]
```

```
fish → zsh → nu → bash  (fish is default)
```

### Migration

`default_shell` is removed from config.toml. Users who had
`default_shell = "nu"` should change to `toggle = ["nu", "bash", "fish"]` (or
whatever shells they want). Backward compatibility: if `default_shell` is
present and `toggle` is not, treat it as `toggle = [default_shell, ...rest]`.

Duplicates in the toggle list are allowed — the list is used as-is with no
deduplication.

## Experiments

### Experiment 1: Add zsh, toggle list, remove default_shell

#### Description

Add zsh as a fourth built-in shell, add the `toggle` config option, and
remove `default_shell`. Small, focused changes to config.rs and main.rs.

#### Changes

**`src/config.rs`**:

- Add zsh to `builtin_shells()` with the fish-style `env` wrapper, `env`
  parser, and `bash` highlighter.
- Replace `default_shell: Option<String>` with `toggle: Option<Vec<String>>`
  in `ShannonConfig`.
- Keep `default_shell` as a deprecated field for backward compat — if present
  and `toggle` is not, convert to a toggle list with that shell first.
- Update `shells()` method:
  - If `toggle` is set: iterate the toggle list, look up each name in
    built-in + custom shells, return in order. Unknown names are skipped
    with a warning.
  - If `toggle` is not set: return all built-in + custom shells (current
    behavior).
  - No deduplication — the list is used as-is.

**`src/main.rs`**:

- Remove `default_shell` references (already removed the env var in previous
  commit).

**Update tests in `src/config.rs`**:

- `test_empty_config` — returns 4 shells (bash, nu, fish, zsh)
- `test_toggle_list` — `toggle = ["nu", "bash"]` returns only those two, nu
  first
- `test_toggle_unknown_shell` — unknown name is skipped
- `test_toggle_with_custom_shell` — custom shell in toggle list works
- `test_default_shell_backward_compat` — `default_shell = "nu"` without
  `toggle` puts nu first
- `test_toggle_duplicates` — `["fish", "bash", "fish"]` returns all three
  entries

**`tests/integration.rs`**:

- Update `zsh` integration tests (same pattern as fish: skip if not
  installed, test echo, env capture, cwd, exit code)

#### Verification

1. `cargo build` succeeds.
2. `cargo test` passes.
3. `cargo run` with no config.toml — bash, nu, fish, zsh all in rotation
   (if installed).
4. Create `config.toml` with `toggle = ["nu", "bash"]` — only nu and bash.
5. Shift+Tab cycles through the toggle list in order.
6. Zsh works: commands execute, env captured, syntax highlighted (via bash
   grammar).

**Result:** Pass

All verification steps confirmed. 66 tests pass (46 unit + 20 integration).
Zsh is a built-in default with 4 integration tests. Toggle list controls the
rotation with 6 config tests covering all cases including duplicates and
unknown shells. Fish command highlighting also fixed (commands now show in
blue).

#### Conclusion

Zsh support and toggle list are complete. Four built-in shells (bash, nu,
fish, zsh) with a configurable rotation via `toggle` in config.toml.
`default_shell` is deprecated but still works as backward compat.

## Conclusion

Issue complete. Shannon now has four built-in shells and a `toggle` config
option that controls the Shift+Tab rotation.

Key changes:
- `src/config.rs` — zsh built-in, `toggle` field, backward compat for
  `default_shell`
- `src/highlighter.rs` — fish command names now colored blue
- `tests/integration.rs` — 4 zsh integration tests
