+++
status = "open"
opened = "2026-03-21"
+++

# Issue 1: Syntax highlighting for bash and nushell

## Goal

Add syntax highlighting to the olshell input line for both bash and nushell,
using tree-sitter grammars. Highlighting should update live as the user types
and switch automatically when the user switches shells with Shift+Tab.

## Background

olshell uses reedline as its line editor. Reedline supports syntax highlighting
via the `Highlighter` trait:

```rust
pub trait Highlighter: Send {
    fn highlight(&self, line: &str, cursor: usize) -> StyledText;
}
```

`StyledText` is a vector of `(Style, String)` pairs. Our implementation needs to
parse the input line, classify tokens (keywords, strings, variables, operators,
comments, etc.), and return styled segments.

### Why tree-sitter

- Grammar-based parsing gives accurate highlighting, not just keyword matching.
- Tree-sitter grammars exist for both bash and nushell.
- Tree-sitter is designed for incremental parsing — fast enough for
  keystroke-by-keystroke highlighting.
- Adding a new shell later means adding a grammar, not writing a new parser.

### Rust crates

- `tree-sitter` — core library with Rust bindings.
- `tree-sitter-bash` — bash grammar.
- `tree-sitter-nu` — nushell grammar (needs verification — may be under a
  different name or repo).

### Open questions

- What crate provides the nushell tree-sitter grammar? Does one exist on
  crates.io, or do we need to build from a git repo?
- What node types do the bash and nushell grammars produce? We need to map
  grammar node types to highlight colors.
- What color scheme should we use? We need a palette that works on both light
  and dark terminals.
- How does tree-sitter handle incomplete/invalid input? The user is typing
  mid-line — the input will often be syntactically incomplete.

### Integration point

In `src/main.rs`, the `build_editor` function creates a `Reedline` instance. We
would add `.with_highlighter(Box::new(highlighter))` where `highlighter` is a
tree-sitter-backed implementation of reedline's `Highlighter` trait. Each shell
gets its own highlighter with its own grammar.
