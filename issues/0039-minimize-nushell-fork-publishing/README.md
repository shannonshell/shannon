+++
status = "open"
opened = "2026-03-31"
+++

# Issue 39: Minimize nushell fork publishing for crates.io

## Goal

Reduce the number of nushell crates Shannon must fork-publish to crates.io to
the absolute minimum. Unmodified crates should come from upstream crates.io.
Only substantively modified crates should be published under a Shannon
namespace.

## Background

Shannon keeps a full nushell fork in `nushell/` (via git subtree) and references
all crates as path dependencies. This is intentional — having the full fork in
the repo allows future modifications to any crate without friction. However,
**publishing** shannonshell to crates.io currently requires publishing every
nushell crate it depends on, because path deps aren't allowed in published
crates.

After removing brush (issue 38), the nushell fork has far fewer modifications
than before. The libc pin relaxation is no longer needed. The only substantive
changes are:

| Crate         | Change                                                                                                  | Severity |
| ------------- | ------------------------------------------------------------------------------------------------------- | -------- |
| `nu-cli`      | ModeDispatcher trait, BashHighlighter, REPL mode dispatch, Shift+Tab, NoOpHighlighter, tree-sitter deps | Heavy    |
| `nu-path`     | Config dir `"nushell"` → `"shannon"` (1 line)                                                           | Trivial  |
| `nu-command`  | 1-line warning fix, removed test-support feature forward                                                | Trivial  |
| `nu-cmd-lang` | Removed test-support feature forward                                                                    | Trivial  |

The `nu-command` and `nu-cmd-lang` changes are build/warning fixes that may not
even be necessary against the crates.io versions. That potentially leaves only
**2 crates** that need fork-publishing: `nu-cli` and `nu-path`.

## Architecture

Shannon's root `Cargo.toml` currently references 15+ nushell crates as path
dependencies plus reedline. The strategy:

1. **Keep the full fork** in `nushell/` for development and future flexibility
2. **Use crates.io versions** for unmodified crates when publishing
3. **Fork-publish only modified crates** under a Shannon namespace (e.g.,
   `shannon-nu-cli`, `shannon-nu-path`)

### Version alignment

The fork is at nushell **0.111.2**. Crates.io currently has **0.111.0**. This
version gap needs investigation:

- Are the 0.111.0 → 0.111.2 changes breaking for Shannon's use?
- Can we pin to 0.111.0 crates.io versions for unmodified deps while our forked
  crates are at 0.111.2?
- Or do we need to sync our fork to match whatever's on crates.io?

### Reedline

Reedline 0.46.0 is on crates.io and Shannon's fork has no code changes. If we
can use the stock crates.io version, that's one fewer crate to publish.

### The nu-path problem

`nu-path` has a 1-line change (`"nushell"` → `"shannon"` for config dir). This
ripples through every crate that calls `nu_config_dir()`. Alternatives to
forking nu-path:

1. **Runtime override**: Set an env var or use a wrapper that redirects the
   config dir at startup, before any nushell code reads it
2. **Symlink**: `~/.config/nushell` → `~/.config/shannon`
3. **Fork-publish**: Publish `shannon-nu-path` (simplest, most explicit)

### Dependency chain concern

If we fork-publish `nu-cli`, it depends on `nu-path`, `nu-protocol`,
`nu-engine`, `nu-parser`, etc. If our forked `nu-cli` references upstream
crates.io versions for those deps, everything must be version-compatible. This
is the main risk area.

### Dual Cargo.toml approach

One option: maintain the path deps for local development (they override
crates.io versions in the workspace) but have the published crate metadata
reference crates.io versions. Cargo supports this via `[dependencies]` with both
`path` and `version` specified — path is used locally, version is used when
publishing.

Example:

```toml
nu-protocol = { version = "0.111.0", path = "nushell/crates/nu-protocol" }
```

This might be the cleanest approach — no changes to the development workflow,
but crates.io publishing uses upstream versions for unmodified deps.

## Open questions

1. Can nushell 0.111.2 forked crates (nu-cli) depend on 0.111.0 crates.io deps
   without breakage?
2. Is the `path` + `version` dual approach sufficient, or do we need separate
   publish manifests?
3. Can the nu-path config dir change be solved at runtime instead of by forking?
4. Do the `nu-command` and `nu-cmd-lang` trivial fixes even matter against
   crates.io 0.111.0?
5. What namespace/naming convention for forked crates? (`shannon-nu-cli` vs
   `nu-cli-shannon` vs something else?)
