+++
status = "open"
opened = "2026-03-22"
+++

# Issue 12: Add zsh as built-in default and toggle list

## Goal

Add zsh as a fourth built-in shell and add a `toggle` config option that lets
users control which shells appear in the Shift+Tab rotation.

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

**Add `toggle` to config.toml:**

```toml
toggle = ["nu", "bash"]
```

If `toggle` is specified, only those shells appear in the Shift+Tab rotation, in
that order. The first shell in the list is the default (unless `default_shell`
is also set, which overrides).

If `toggle` is omitted, all installed built-in + custom shells are available
(current behavior).

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
nu → bash  (only these two)
```

Toggle list with default override:

```toml
default_shell = "bash"
toggle = ["nu", "bash", "fish"]
```

```
bash → nu → fish  (bash moved to front)
```
