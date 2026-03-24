+++
status = "closed"
opened = "2026-03-23"
closed = "2026-03-23"
+++

# Issue 21: Add background color support to theme color syntax

## Goal

Add `--background=color` syntax to `parse_style` so users can set background colors in
config.toml. Currently our default `ai_badge` style (black text on magenta
background) is inexpressible in our own config language.

## Background

`parse_style` in `src/theme.rs` parses color strings like `"green --bold"` or
`"#FF79C6 --italic"`. It supports foreground color and modifiers (bold, italic,
underline, reverse, etc.) but has no way to set a background color.

The `ai_badge` default is `Style::new().fg(Color::Black).on(Color::Magenta)` —
this cannot be expressed in config.toml. A user who wants to change the AI badge
background color has no way to do so.

### Proposed syntax

```
"black --background=magenta"            ← black text on magenta background
"white --bold --background=#1e1e2e"    ← bold white text on hex background
"--background=red"                     ← default fg on red background
```

The `--background=` prefix mirrors nu_ansi_term's `.on()` method and matches fish's `--background=` syntax.

### What changes

- `parse_style` in `src/theme.rs` — detect `--background=color` in the modifiers and
  call `.on(parse_nu_color(color))` on the style
- Update docs to mention `--background=` syntax
- The default `ai_badge` becomes expressible as `"black --background=magenta"`

## Experiments

### Experiment 1: Add --background= to parse_style

#### Description

Add `--background=color` support to `parse_style` in `src/theme.rs`. Small
change — detect the prefix in the modifier loop, parse the color, call `.on()`.

#### Changes

**`shannon/src/theme.rs`** — update `parse_style`:

In the modifier loop, before the `match`, check for `--background=`:

```rust
if let Some(bg) = part.strip_prefix("--background=") {
    style = style.on(parse_nu_color(bg));
} else {
    match *part { ... }
}
```

Add test:

- `test_parse_style_background` — `"black --background=magenta"` produces
  `Style::new().fg(Color::Black).on(Color::Magenta)`
- `test_parse_style_background_hex` — `"white --background=#1e1e2e"` works
- `test_parse_style_background_only` — `"--background=red"` sets bg with
  default fg

**`docs/reference/02-configuration.md`** — update color values section to
mention `--background=`.

#### Verification

1. `cargo build` succeeds.
2. `cargo test` passes.
3. Set `ai_badge = "black --background=magenta"` in config.toml — same
   appearance as the default.
4. Set `ai_badge = "white --bold --background=blue"` — white bold on blue.
5. Existing theme colors without `--background=` still work unchanged.

**Result:** Pass

91 tests pass. `--background=` works with named colors and hex. Docs updated.

## Conclusion

Issue complete. `parse_style` now supports `--background=color`, matching
fish's syntax. The default `ai_badge` is expressible as
`"black --background=magenta"` in config.toml.
