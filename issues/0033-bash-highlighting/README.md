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
