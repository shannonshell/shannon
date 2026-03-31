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

**Result:** Pass

Answers:

1. **Can forked nu-cli use upstream 0.111.0 deps?** Not as-is — our fork is
   based on upstream 0.111.2, which has breaking API changes (GenericError
   refactor). However, Shannon's own additions use only 0.111.0-era APIs. We can
   rebase our changes onto stock 0.111.0.
2. **Path + version dual approach?** Works once we're on 0.111.0.
3. **Nu-path runtime override?** No mechanism exists. Fork-publish required.
4. **Nu-command / nu-cmd-lang fixes?** Already reverted — no Shannon changes
   remain in those crates.
5. **Stock reedline?** Yes — `bashisms` feature exists in crates.io 0.46.0.
6. **Dead brush crates?** Yank from crates.io.

#### Conclusion

Minimum fork set is 2 crates: `shannon-nu-cli` and `shannon-nu-path`. But we
must first rebase our nushell fork onto 0.111.0 so our nu-cli changes compile
against crates.io APIs. Experiment 2 does this.

### Experiment 2: Rebase nushell fork onto 0.111.0

Replace the current nushell subtree (based on post-0.111.0 upstream main) with
stock nushell 0.111.0 (the crates.io release), then re-apply only Shannon's
changes on top.

#### Why

Our fork is at 0.111.2, which includes ~90 upstream commits after 0.111.0 with
breaking API changes. Shannon's own changes don't use any of those new APIs.
Rebasing onto 0.111.0 aligns our fork with crates.io, so unmodified crates can
come from upstream.

#### Changes

**Step 1: Move the current nushell subtree aside**

Rename so we have the current code as reference while working:

```sh
git mv nushell nushell-old
git commit -m "Move nushell aside for rebase"
```

**Step 2: Add stock nushell 0.111.0 subtree**

```sh
git subtree add --prefix nushell upstream-nushell 0.111.0
```

Now we have both `nushell/` (clean 0.111.0) and `nushell-old/` (our 0.111.2 fork
with Shannon changes) side by side.

**Step 3: Re-apply Shannon's nu-cli changes**

Port changes from `nushell-old/crates/nu-cli/` to `nushell/crates/nu-cli/`,
using the old files as reference.

Apply these changes to `nushell/crates/nu-cli/`:

1. **New file: `src/mode_dispatcher.rs`** — Copy from our current fork. Uses
   only `HashMap` and `PathBuf`, no nushell APIs. No changes needed.

2. **New file: `src/bash_highlight.rs`** — Copy from our current fork. Uses
   `nu_color_config::get_shape_color`, `nu_protocol::Config`,
   `reedline::Highlighter` — all exist in 0.111.0.

3. **Modify: `src/lib.rs`** — Add `mod bash_highlight`, `mod mode_dispatcher`,
   and their `pub use` exports. The 0.111.0 lib.rs won't have the `hints` mod
   that 0.111.2 added, so the diff is cleaner.

4. **Modify: `src/repl.rs`** — Add Shannon's ~100 lines:
   - Import `mode_dispatcher::ModeDispatcher`
   - `evaluate_repl()` gains `mode_dispatcher` param
   - SHANNON_MODE check + dispatch block in the command execution section
   - Highlighter swapping (BashHighlighter vs NuHighlighter based on mode)
   - `__shannon_switch` handler in the host command section
   - Shift+Tab keybinding registration

   **Key:** The 0.111.0 repl.rs won't have ExternalHinter or other 0.111.2
   additions. Apply Shannon's changes to the 0.111.0 code, not the 0.111.2 code.

5. **Modify: `src/nu_highlight.rs`** — Add `NoOpHighlighter` struct.

6. **Modify: `Cargo.toml`** — Add `tree-sitter`, `tree-sitter-bash`,
   `tree-sitter-language` dependencies.

**Step 3: Re-apply Shannon's nu-path change**

In `nushell/crates/nu-path/src/helpers.rs`, change `p.push("nushell")` to
`p.push("shannon")`.

**Step 4: Update Shannon's root Cargo.toml**

Change all nushell path deps to include both `version` and `path`:

```toml
nu-cli = { version = "0.111.0", path = "nushell/crates/nu-cli" }
nu-protocol = { version = "0.111.0", path = "nushell/crates/nu-protocol" }
```

Change reedline to crates.io:

```toml
reedline = { version = "0.46.0", features = ["sqlite", "bashisms"] }
```

Keep `path` for local development (path takes precedence). The `version` is used
when publishing to crates.io.

**Step 6: Verify local build uses path deps**

`cargo build` should still use local path deps and compile everything from the
nushell subtree, same as today.

**Step 7: Delete nushell-old**

```sh
git rm -r nushell-old/
git commit -m "Remove old nushell subtree"
```

#### Verification

1. `cargo build` — compiles successfully against 0.111.0 nushell
2. `cargo test` — all tests pass
3. Manual test: `shannon` → bash mode → `echo hello` → works
4. Manual test: `export FOO=bar` → `echo $FOO` → `bar`
5. Manual test: `nvm install 24` → completes
6. Manual test: Shift+Tab mode switching works
7. Manual test: bash syntax highlighting works
8. `grep -r "0.111.2" nushell/` — no 0.111.2 references remain
