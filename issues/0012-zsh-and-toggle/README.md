+++
status = "open"
opened = "2026-03-22"
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
