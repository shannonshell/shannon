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

## Prior art

We already publish 30+ `shannon-*` crates to crates.io (e.g., `shannon-nu-cli`,
`shannon-nu-protocol`, `shannon-nu-path`, etc.) at versions 0.111.3–0.111.4. We
also published `shannon-brush-core`, `shannon-brush-builtins`, and
`shannon-brush-parser` — those are now dead since brush was removed.

The current state publishes **everything** as forked crates. The goal is to stop
publishing unmodified crates and use upstream versions from crates.io instead.

Upstream crates.io has nushell crates at **0.111.0**. Our fork is at
**0.111.2**. Reedline **0.46.0** matches crates.io exactly.

## Open questions

1. Can our forked `shannon-nu-cli` depend on upstream `nu-protocol = "0.111.0"`
   (crates.io) instead of `shannon-nu-protocol = "0.111.4"`? Are there breaking
   API changes between 0.111.0 and 0.111.2?
2. Is the `path` + `version` dual approach sufficient, or do we need separate
   publish manifests?
3. Can the nu-path config dir change be solved at runtime instead of by forking?
4. Do the `nu-command` and `nu-cmd-lang` trivial fixes even matter against
   crates.io 0.111.0?
5. Can we use stock `reedline = "0.46.0"` from crates.io?
6. What happens to the dead `shannon-brush-*` crates? (Yank them?)

## Experiments

### Experiment 1: Audit version compatibility and determine minimum fork set

Before changing any code, answer all open questions by examining the actual
crate APIs and testing version compatibility.

#### Steps

**Step 1: Check if our nushell fork can build against upstream 0.111.0 deps**

In a temporary branch, change Shannon's `Cargo.toml` to use crates.io versions
for all unmodified crates while keeping path deps for `nu-cli` and `nu-path`.
Attempt `cargo build`. Record which crates fail and why.

Specifically, test this Cargo.toml pattern for unmodified crates:

```toml
nu-protocol = { version = "0.111.0", path = "nushell/crates/nu-protocol" }
```

The `path` takes precedence locally, but `version` is what gets published. The
question is whether `nu-cli` (0.111.2 in our fork) can compile against
`nu-protocol` 0.111.0 APIs.

To actually test crates.io resolution (not path override), temporarily remove
the `path` for one unmodified crate and see if it resolves from crates.io and
links cleanly.

**Step 2: Check reedline compatibility**

Test whether stock `reedline = "0.46.0"` from crates.io works. Our fork has no
code changes, but we need to confirm the `bashisms` feature exists in the
published version.

**Step 3: Check nu-path runtime alternative**

Read `nushell/crates/nu-path/src/helpers.rs` to see if `nu_config_dir()` checks
any env var before defaulting to `"shannon"` (our change) or `"nushell"`
(upstream). If not, evaluate whether we can add one and upstream it, or if
fork-publishing `shannon-nu-path` is simpler.

**Step 4: Check nu-command and nu-cmd-lang trivial fixes**

Read the specific lines we changed in `nu-command` and `nu-cmd-lang`. Check
whether the upstream 0.111.0 crates.io versions have the same issues, or if our
fixes were for problems specific to our workspace configuration.

#### Deliverable

A clear answer for each open question, plus the definitive minimum set of crates
that need fork-publishing.

#### Verification

All questions answered with evidence (build output, source code, crates.io
metadata). No guessing.
