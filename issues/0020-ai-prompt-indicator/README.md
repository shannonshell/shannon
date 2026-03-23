+++
status = "open"
opened = "2026-03-23"
+++

# Issue 20: Highlighted AI prompt indicator

## Goal

When AI mode is active, show a highlighted `AI` badge at the start of the
prompt. It must be visually obvious because AI mode sends input to a third party
service.

## Background

Currently AI mode shows `[nu:ai]` — the `:ai` suffix is easy to miss. The user
needs a clear visual signal that their input is going to an LLM, not to their
shell.

### Design

Normal mode:

```
[nu] ~/project >
```

AI mode:

```
AI [nu] ~/project >
^^
highlighted: magenta background, dark text
```

The `AI` badge uses a colored background (ANSI reverse or explicit bg) to make
it impossible to miss. It appears at the very start of the prompt, before the
shell name.

### Implementation

**`src/prompt.rs`** — update `render_prompt_left`:

In AI mode, prepend `AI` with embedded ANSI codes for background color. After
the badge, reset and re-apply the prompt foreground color so the rest of the
prompt renders normally.

**`src/theme.rs`** — add `ai_badge` style:

A `Style` for the AI badge with reverse magenta by default
(`Color::Magenta.reverse()`). This uses the terminal's magenta, so it adapts to
the terminal theme.

**`src/config.rs`** — add `ai_badge: Option<String>` to ThemeConfig.

Users can override: `ai_badge = "yellow --reverse"` in config.toml.
