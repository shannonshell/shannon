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

## Experiments

### Experiment 1: Highlighted AI badge

#### Description

Add a highlighted `AI` badge at the start of the prompt in AI mode. Use
embedded ANSI codes in `render_prompt_left()` since reedline's Prompt trait
only supports a single foreground color. Add `ai_badge` to the theme system.

#### Changes

**`shannon/src/theme.rs`**:

Add `pub ai_badge: nu_ansi_term::Style` to Theme struct.

Default: `Style::new().fg(Color::Black).on(Color::Magenta)` — black text
on magenta background. Uses ANSI colors so it adapts to the terminal theme.

In `Theme::from_config`, apply override if `config.ai_badge` is set.

In `apply_named_theme`, no fish theme maps to this (fish has no AI mode),
so the default always applies unless user overrides.

**`shannon/src/config.rs`**:

Add `pub ai_badge: Option<String>` to ThemeConfig.

**`shannon/src/prompt.rs`**:

Add `pub ai_badge_style: nu_ansi_term::Style` to ShannonPrompt.

Update `render_prompt_left()`:

```rust
if self.ai_mode {
    let badge = self.ai_badge_style.paint("AI").to_string();
    Cow::Owned(format!(
        "{} [{}] {}",
        badge,
        self.shell_name,
        self.tilde_contract()
    ))
} else {
    Cow::Owned(format!(
        "[{}] {}",
        self.shell_name,
        self.tilde_contract()
    ))
}
```

The `Style::paint()` produces ANSI escape codes inline. Reedline renders
them as-is. After the badge, reedline applies `get_prompt_color()` to
the rest of the prompt text, so the `[nu] ~/project` part stays in the
normal prompt color.

**`shannon/src/repl.rs`**:

Pass `theme.ai_badge` to ShannonPrompt:

```rust
ai_badge_style: theme.ai_badge,
```

#### Verification

1. `cargo build` succeeds.
2. `cargo test` passes.
3. Normal mode: `[nu] ~/project >` — no AI badge.
4. AI mode (Enter on empty): `AI [nu] ~/project >` with AI having a
   colored background.
5. The rest of the prompt (`[nu] ~/project >`) stays in normal prompt color.
6. Badge adapts to terminal theme (ANSI magenta).
7. Override works: `ai_badge = "yellow --reverse"` in config.toml.
