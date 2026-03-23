+++
status = "open"
opened = "2026-03-23"
+++

# Issue 21: Add background color support to theme color syntax

## Goal

Add `--on=color` syntax to `parse_style` so users can set background colors
in config.toml. Currently our default `ai_badge` style (black text on magenta
background) is inexpressible in our own config language.

## Background

`parse_style` in `src/theme.rs` parses color strings like `"green --bold"` or
`"#FF79C6 --italic"`. It supports foreground color and modifiers (bold, italic,
underline, reverse, etc.) but has no way to set a background color.

The `ai_badge` default is `Style::new().fg(Color::Black).on(Color::Magenta)` —
this cannot be expressed in config.toml. A user who wants to change the AI
badge background color has no way to do so.

### Proposed syntax

```
"black --on=magenta"           ← black text on magenta background
"white --bold --on=#1e1e2e"    ← bold white text on hex background
"--on=red"                     ← default fg on red background
```

The `--on=` prefix mirrors nu_ansi_term's `.on()` method and is consistent
with our existing `--modifier` syntax.

### What changes

- `parse_style` in `src/theme.rs` — detect `--on=color` in the modifiers
  and call `.on(parse_nu_color(color))` on the style
- Update docs to mention `--on=` syntax
- The default `ai_badge` becomes expressible as `"black --on=magenta"`
