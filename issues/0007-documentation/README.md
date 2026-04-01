+++
status = "closed"
opened = "2026-03-21"
closed = "2026-04-01"
+++

# Issue 7: Documentation system

## Goal

Establish a documentation system using markdown files that can serve as both the
project's reference docs and the source for a future website. Every feature
should be documented — new features shouldn't ship without docs.

## Background

shannon currently has a README.md with a high-level overview and a CLAUDE.md
with architecture notes. But there is no user-facing documentation explaining
how to use shannon, what features it has, or how it works.

We need a `docs/` directory with markdown files that:

1. Are readable on their own (browsable on GitHub).
2. Can be processed by a static site generator (mdBook, Zola, Hugo, etc.) to
   produce a website later.
3. Cover all existing and future features.
4. Follow a consistent structure so new docs are easy to add.

### What needs documenting

**Getting started:**

- Installation
- First run
- Switching shells with Shift+Tab

**Features:**

- Shell switching (Shift+Tab)
- Environment variable synchronization
- Working directory synchronization
- Exit code propagation
- Syntax highlighting (Tokyo Night)
- Per-shell command history
- Ctrl+R reverse search

**Reference:**

- Supported shells
- Keybindings
- Configuration (~/.config/shannon/)
- Architecture overview (how wrapper scripts work, strings-only policy)

**Future features (document as implemented):**

- Tab completion
- Autosuggestions
- Git branch in prompt
- Config files
- Vi mode

### Static site generator choice

Defer the choice of static site generator. For now, just write well-structured
markdown in `docs/` with a clear directory layout. The markdown should be
generator-agnostic — plain markdown with standard frontmatter. Any generator
(mdBook, Zola, Hugo) can consume it later.

### Proposed structure

```
docs/
├── README.md              ← index / landing page
├── getting-started.md     ← installation + first run
├── features/
│   ├── shell-switching.md
│   ├── env-sync.md
│   ├── syntax-highlighting.md
│   ├── history.md
│   └── ...
├── reference/
│   ├── keybindings.md
│   ├── configuration.md
│   ├── supported-shells.md
│   └── architecture.md
└── contributing.md        ← how to add features + docs
```

### Documentation principles

- **Every feature gets a doc.** No feature ships without a corresponding
  markdown file.
- **User-first language.** Write for someone who just installed shannon, not for
  contributors reading the source.
- **Show, don't tell.** Use concrete examples and terminal output snippets.
- **Keep docs in sync.** When a feature changes, its doc changes in the same
  commit.

## Experiments

### Experiment 1: Feature inventory and doc plan

#### Description

Audit every user-facing feature currently implemented in shannon and map each
one to a documentation file. The issue's "What needs documenting" section was
written before tab completion and bracketed paste were added — this experiment
produces the accurate, current list.

#### Feature Inventory

Audited from source code on 2026-03-21:

| Feature                   | Files                  | Key details                                                                          |
| ------------------------- | ---------------------- | ------------------------------------------------------------------------------------ |
| Shell switching           | main.rs                | Shift+Tab cycles Bash → Nushell → Bash. Auto-detects installed shells.               |
| Env var synchronization   | executor.rs            | Captured via wrapper scripts. Strings only — nushell arrays joined with `:`.         |
| Working directory sync    | executor.rs            | Captured alongside env vars. Tilde-contracted in prompt.                             |
| Exit code propagation     | executor.rs, prompt.rs | Prompt shows `>` on success, `!` on failure.                                         |
| Syntax highlighting       | highlighter.rs         | Tree-sitter grammars for bash and nushell. Tokyo Night color scheme.                 |
| Per-shell command history | main.rs, shell.rs      | FileBackedHistory, 10k entries, stored in `~/.config/shannon/`.                      |
| Ctrl+R reverse search     | main.rs (reedline)     | Built into reedline's default emacs keybindings.                                     |
| Tab completion            | completer.rs           | File/directory completion. Hidden files excluded unless `.` prefix. Tilde expansion. |
| Bracketed paste           | main.rs                | Enabled via reedline. Pasted text treated as atomic input.                           |
| Ctrl+L clear screen       | main.rs (reedline)     | Built into reedline's default emacs keybindings.                                     |
| Ctrl+C interrupt          | main.rs                | Re-prompts during input. Forwarded to subprocess during execution.                   |
| Ctrl+D exit               | main.rs                | Exits shannon cleanly.                                                               |
| Emacs keybindings         | main.rs (reedline)     | Standard emacs line editing (Ctrl+A/E/U/K/W/Y, Alt+B/F, etc.).                       |
| Prompt display            | prompt.rs              | Shows `[shell] ~/path >` with shell-colored name.                                    |

#### Updated Doc Structure

Based on the inventory, the proposed structure needs minor updates. Tab
completion moves from "future" to "features". Bracketed paste and screen control
don't need their own pages — they're standard behavior mentioned in the
keybindings reference.

```
docs/
├── README.md                      ← index with links to all pages
├── getting-started.md             ← install, first run, shell switching intro
├── features/
│   ├── shell-switching.md         ← Shift+Tab, auto-detection, rotation
│   ├── state-sync.md              ← env vars, cwd, exit code (combined)
│   ├── syntax-highlighting.md     ← tree-sitter, Tokyo Night, per-shell colors
│   ├── history.md                 ← per-shell history, Ctrl+R, file locations
│   └── tab-completion.md          ← file/dir completion, hidden files, tilde
├── reference/
│   ├── keybindings.md             ← all keybindings in one table
│   ├── configuration.md           ← ~/.config/shannon/, history files
│   └── supported-shells.md        ← bash, nushell, how to add more
└── architecture.md                ← wrapper scripts, strings-only, subprocess model
```

Changes from the original proposal:

- **Merged** env-sync, cwd-sync, exit code into single `state-sync.md` — they
  use the same mechanism and are better explained together.
- **Added** `tab-completion.md` — now implemented.
- **Dropped** `contributing.md` — premature, revisit when there are external
  contributors.
- **Moved** `architecture.md` to top level — it's neither a feature nor a
  reference, it's background.

#### Verification

1. Every implemented feature maps to at least one doc file.
2. No doc file covers unimplemented features (AI mode, config files, etc.).
3. The structure is generator-agnostic plain markdown.

**Result:** Pass

The inventory is complete and the doc structure is updated.

#### Conclusion

We have 14 user-facing features, all mapped to 10 documentation files across 3
categories (getting started, features, reference) plus an architecture page.
Ready to write the docs in Experiment 2.

### Experiment 2: Write all documentation

#### Description

Create the `docs/` directory and write all 10 documentation files from the
structure defined in Experiment 1. Each file follows the documentation
principles: user-first language, concrete examples with terminal snippets, no
unimplemented features.

#### Changes

**`docs/README.md`** — Index page. One-paragraph description of shannon, then a
bulleted list of links to every doc page organized by category (Getting Started,
Features, Reference, Architecture).

**`docs/getting-started.md`** — Installation (requires Rust,
`cargo build
--release`), first run (`cargo run` or path to binary), what you see
on launch (the `[bash]` prompt), switching shells with Shift+Tab, running your
first command, exiting with Ctrl+D or `exit`.

**`docs/features/shell-switching.md`** — How Shift+Tab works, the rotation
order, auto-detection of installed shells, what happens when a shell isn't
installed, state preservation across switches, example session showing a switch
from bash to nushell.

**`docs/features/state-sync.md`** — Environment variables, working directory,
and exit code synchronization. How the wrapper script mechanism works (high
level — point to architecture.md for details). What transfers and what doesn't
(strings only — no bash arrays or nushell tables). Example: export a variable in
bash, switch to nushell, use it. Example: `cd` in one shell, switch, see the new
cwd.

**`docs/features/syntax-highlighting.md`** — Tokyo Night theme, per-shell
grammars (bash and nushell), what gets colored (keywords, strings, variables,
commands, comments, errors, etc.), how incomplete input is handled (graceful
fallback). Include a color reference table.

**`docs/features/history.md`** — Per-shell history files, storage location
(`~/.config/shannon/bash_history`, `nu_history`), 10k entry limit, Ctrl+R
reverse search, up/down arrow navigation, history isolation between shells.

**`docs/features/tab-completion.md`** — File and directory completion, how to
trigger (Tab key), behavior on single match vs multiple matches, directory
trailing slash, hidden file filtering (type `.` to see them), tilde expansion,
columnar menu display.

**`docs/reference/keybindings.md`** — Complete table of all keybindings:
Shift+Tab (shell switch), Tab (completion), Ctrl+R (history search), Ctrl+L
(clear screen), Ctrl+C (interrupt), Ctrl+D (exit), Ctrl+A/E (line start/end),
Ctrl+U/K (kill line), Ctrl+W (kill word), Ctrl+Y (yank), Alt+B/F (word
movement), and other emacs defaults.

**`docs/reference/configuration.md`** — Config directory location
(`~/.config/shannon/`), what's stored there today (history files only), planned
future configuration (mention briefly without documenting unimplemented
features).

**`docs/reference/supported-shells.md`** — Bash and nushell: version
requirements, what works, known limitations per shell. Brief note on what adding
a new shell would require (wrapper script + parser in executor.rs, grammar in
highlighter.rs).

**`docs/architecture.md`** — The subprocess-per-command model, wrapper scripts,
env capture and parsing, the strings-only boundary, why no persistent sessions.
Aimed at someone who wants to understand how shannon works under the hood, not
just how to use it.

#### Verification

1. All 10 files exist in the correct directory structure.
2. Every feature from the Experiment 1 inventory is covered in at least one doc
   file.
3. No doc file references unimplemented features as if they exist (AI mode,
   config files, vi mode, etc.).
4. Each doc file has at least one concrete example or terminal snippet.
5. All internal links between docs are correct.
6. Files are plain markdown with no generator-specific syntax.

**Result:** Pass (superseded)

The docs were written but later replaced by a full Astro website at `website/`.

## Conclusion

Documentation system is live. The original plan called for markdown files in
`docs/` — instead, Shannon got a full Astro static site deployed to Cloudflare
Pages. The website covers getting started, features, and reference docs. The
markdown-first approach from Experiment 1 informed the site structure.
