+++
status = "closed"
opened = "2026-03-21"
closed = "2026-03-21"
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

## Experiments

### Experiment 1: Research tree-sitter crates and grammar node types

#### Description

Answer all open questions before writing any integration code. We need to:

1. Find the correct crate for the nushell tree-sitter grammar (check crates.io
   and GitHub).
2. Write a throwaway Rust program that parses sample bash and nushell input with
   tree-sitter and prints the syntax tree. This tells us the node types each
   grammar produces and how incomplete input is handled.
3. Document the node-type-to-color mapping we'll use.
4. Choose a color scheme.

#### Changes

- Create `experiments/0001-tree-sitter-research/` with a standalone `Cargo.toml`
  and `src/main.rs` that:
  - Depends on `tree-sitter`, `tree-sitter-bash`, and whatever the nushell
    grammar crate is.
  - Parses several sample inputs for each shell:
    - Complete command: `echo "hello world"`
    - Pipeline: `ls -la | grep foo`
    - Variable: `export FOO=bar` (bash), `$env.FOO = "bar"` (nushell)
    - Incomplete input: `echo "unterminated`
    - Comment: `# this is a comment`
  - Prints the full S-expression tree for each parse.
  - Lists all unique node type names found.

#### Verification

1. `cd experiments/0001-tree-sitter-research && cargo run` succeeds.
2. Output shows the S-expression tree for each sample input.
3. We can see how tree-sitter represents incomplete/invalid input (look for
   `ERROR` or `MISSING` nodes).
4. We have a complete list of node types for both grammars.
5. Document findings in this issue (as the result).

**Result:** Pass

All open questions answered:

1. **Nushell grammar crate:** `tree-sitter-nu` from
   `github.com/nushell/tree-sitter-nu` (git dependency — not on crates.io).
   Works out of the box with `tree-sitter = "0.26"`.

2. **Incomplete input handling:** Bash uses `ERROR` nodes, nushell uses
   `MISSING` nodes. Both set `has_error: true` on the root. The rest of the tree
   remains valid — tree-sitter recovers gracefully.

3. **Color scheme:** Tokyo Night.

4. **Node type to highlight category mapping:**

   | Highlight category | Bash node types                                                 | Nushell node types                                             | Tokyo Night color    |
   | ------------------ | --------------------------------------------------------------- | -------------------------------------------------------------- | -------------------- |
   | Keyword            | `if`, `then`, `else`, `fi`, `for`, `in`, `do`, `done`, `export` | `if`, `else`, `for`, `in`, `let`, `def`, `where`, `true`       | Purple `#bb9af7`     |
   | Command name       | `command_name`, `word` (first child of `command`)               | `cmd_identifier`                                               | Blue `#7aa2f7`       |
   | String             | `string`, `string_content`, `heredoc_body`                      | `val_string`, `string_content`, `escaped_interpolated_content` | Green `#9ece6a`      |
   | Number             | `number`                                                        | `val_number`                                                   | Orange `#ff9e64`     |
   | Variable           | `variable_name`, `simple_expansion`                             | `val_variable`, `identifier` (in `$env.X` context)             | Cyan `#7dcfff`       |
   | Operator / Pipe    | `\|`, `>`, `<`, `=`                                             | `\|`, `>`, `+`, `=`, `..`                                      | Magenta `#c0caf5`    |
   | Comment            | `comment`                                                       | `comment`                                                      | Gray `#565f89`       |
   | Type               | —                                                               | `flat_type`, `param_type`                                      | Yellow `#e0af68`     |
   | Boolean            | —                                                               | `val_bool`, `true`                                             | Orange `#ff9e64`     |
   | Error              | `ERROR`                                                         | `MISSING`                                                      | Red `#f7768e`        |
   | Default            | everything else                                                 | everything else                                                | Foreground `#a9b1d6` |

#### Conclusion

tree-sitter is viable for both shells. The grammars produce rich, well-labeled
node types. Incomplete input is handled gracefully. We have a clear color
mapping for Tokyo Night. Ready to implement the reedline `Highlighter` trait
backed by tree-sitter in Experiment 2.

### Experiment 2: Implement tree-sitter highlighter for reedline

#### Description

Implement the reedline `Highlighter` trait using tree-sitter grammars and the
Tokyo Night color mapping from Experiment 1. The highlighter parses the input
line on every keystroke, walks the syntax tree, and returns styled segments.

Each shell gets its own highlighter instance with its own grammar and
node-type-to-color mapping. The highlighter is swapped when the user switches
shells with Shift+Tab (already handled — `build_editor` is called per shell).

#### Changes

**`Cargo.toml`** — add dependencies:

- `tree-sitter = "0.26"`
- `tree-sitter-bash = "0.23"`
- `tree-sitter-nu = { git = "https://github.com/nushell/tree-sitter-nu" }`

**`src/highlighter.rs`** (new file):

- `TreeSitterHighlighter` struct holding a `tree_sitter::Parser` and the
  `ShellKind` (to select the color mapping).
- `TreeSitterHighlighter::new(shell: ShellKind) -> Self` — creates the parser
  and sets the language.
- `impl Highlighter for TreeSitterHighlighter` — the `highlight` method:
  1. Parse the input line with tree-sitter.
  2. Walk the tree with a `TreeCursor`, visiting every leaf node.
  3. For each leaf node, map its `kind()` to a Tokyo Night color based on the
     shell-specific mapping table from Experiment 1.
  4. Build a `StyledText` from the styled segments.
  5. Any bytes not covered by a leaf node get the default foreground color.
- Helper function `node_style(shell: ShellKind, kind: &str) -> Style` — returns
  the `nu_ansi_term::Style` for a given node type.
- Tokyo Night colors as constants (using `nu_ansi_term::Color::Rgb`).

**`src/main.rs`** — update `build_editor`:

- Add `mod highlighter;`
- In `build_editor`, create a `TreeSitterHighlighter` for the active shell and
  pass it via `.with_highlighter(Box::new(highlighter))`.

#### Verification

1. `cargo build` succeeds.
2. `cargo run`, type `echo "hello world"` in bash — `echo` is blue,
   `"hello
   world"` is green.
3. Type `# comment` — gray text.
4. Type `export FOO=bar` — `export` is purple, `FOO` is cyan.
5. Shift+Tab to nushell, type `let x = 42` — `let` is purple, `42` is orange.
6. Type `ls | where size > 1kb` — `ls` and `where` colored appropriately.
7. Type an incomplete string `echo "unterminated` — error portion is red.
8. Switching shells changes the highlighting rules immediately.

**Result:** Pass

All verification steps confirmed by the user. Syntax highlighting works for both
bash and nushell with Tokyo Night colors. Switching shells with Shift+Tab swaps
the highlighter immediately.

#### Conclusion

Tree-sitter integration is complete. The `TreeSitterHighlighter` in
`src/highlighter.rs` implements reedline's `Highlighter` trait, parses input on
each keystroke, walks leaf nodes, and maps them to Tokyo Night colors. Adding
highlighting for a new shell means adding a color mapping function and a
tree-sitter grammar dependency.

## Conclusion

Issue complete. Syntax highlighting for bash and nushell is implemented using
tree-sitter grammars and the Tokyo Night color scheme. Key files:

- `src/highlighter.rs` — `TreeSitterHighlighter` struct and color mappings
- `Cargo.toml` — `tree-sitter`, `tree-sitter-bash`, `tree-sitter-nu` deps
- `experiments/0001-tree-sitter-research/` — research program (throwaway)
