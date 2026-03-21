+++
status = "open"
opened = "2026-03-21"
+++

# Issue 7: Documentation system

## Goal

Establish a documentation system using markdown files that can serve as both the
project's reference docs and the source for a future website. Every feature
should be documented — new features shouldn't ship without docs.

## Background

olshell currently has a README.md with a high-level overview and a CLAUDE.md
with architecture notes. But there is no user-facing documentation explaining
how to use olshell, what features it has, or how it works.

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
- Configuration (~/.config/olshell/)
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
- **User-first language.** Write for someone who just installed olshell, not for
  contributors reading the source.
- **Show, don't tell.** Use concrete examples and terminal output snippets.
- **Keep docs in sync.** When a feature changes, its doc changes in the same
  commit.
