+++
status = "open"
opened = "2026-03-26"
+++

# Issue 33: Bash syntax highlighting in brush mode

## Goal

Restore syntax highlighting for brush mode using tree-sitter-bash. Currently
brush mode uses `NoOpHighlighter` which shows plain unstyled text.

## Background

Shannon previously had tree-sitter-bash highlighting in its custom REPL. When we
rearchitected to use nushell's REPL (issue 32), we replaced nushell's
`NuHighlighter` with `NoOpHighlighter` for non-nu modes to avoid false syntax
errors on bash code. This fixed the errors but removed all highlighting.

The fix: create a `BashHighlighter` that implements reedline's `Highlighter`
trait using tree-sitter-bash. Use it in brush mode. AI mode stays with
`NoOpHighlighter` (plain text is appropriate for chat).

### Implementation

The `BashHighlighter` lives in the nushell fork (alongside `NuHighlighter` in
nu-cli) so it can be used directly in `loop_iteration()`. It needs:

- `tree-sitter` and `tree-sitter-bash` as dependencies of nu-cli
- A `BashHighlighter` struct implementing `reedline::Highlighter`
- Color scheme matching nushell's theme (or configurable)

The mode check in `loop_iteration()` already exists — just swap
`NoOpHighlighter` for `BashHighlighter` when the mode is "brush".

## Experiments

### Experiment 1: BashHighlighter in nu-cli using tree-sitter-bash

#### Description

Create a `BashHighlighter` in the nushell fork's nu-cli crate. Port the
bash-specific highlighting logic from shannon's old `highlighter.rs` (deleted in
issue 32, recoverable from git history at commit `39571bb^`).

The old highlighter was 278 lines supporting bash, nushell, and fish. We only
need the bash path (~80 lines of node matching + ~70 lines of tree walking).

#### Changes

**`nushell/crates/nu-cli/Cargo.toml`:**

- Add `tree-sitter = "0.26"` and `tree-sitter-bash = "0.23"` deps
- Add `nu-ansi-term = { workspace = true }` (for `Style`/`Color`)

**`nushell/crates/nu-cli/src/bash_highlight.rs`** (new file):

- `BashHighlighter` struct with color fields
- `impl Highlighter for BashHighlighter` using tree-sitter-bash
- `bash_color()` function mapping node kinds to colors
- `collect_leaf_styles()` tree walker
- Colors derived from nushell's config `color_config` or sensible defaults

**`nushell/crates/nu-cli/src/repl.rs`:**

- In the mode check (line ~409), replace `NoOpHighlighter` with
  `BashHighlighter` for brush mode

**`nushell/crates/nu-cli/src/lib.rs`:**

- Add `mod bash_highlight;`

#### Verification

1. `cargo build` succeeds.
2. Brush mode: keywords (`if`, `for`, `export`) colored.
3. Brush mode: strings colored.
4. Brush mode: variables (`$FOO`) colored.
5. Brush mode: comments (`# hello`) colored.
6. Nu mode: highlighting unchanged (still uses NuHighlighter).
7. AI mode: no highlighting (still uses NoOpHighlighter).
