+++
status = "closed"
opened = "2026-03-21"
closed = "2026-03-21"
+++

# Issue 5: Bracketed paste

## Goal

Verify and ensure bracketed paste works in shannon.

## Background

Bracketed paste prevents pasted text from being interpreted as keystrokes
character-by-character. Without it, pasting a multi-line script could trigger
partial execution. All modern shells support this.

Reedline uses crossterm for terminal input, and crossterm supports bracketed
paste mode. This likely works already.

### Verification steps

1. Run `cargo run`.
2. Copy a multi-line string and paste it into the prompt.
3. If the entire string appears as input (not executed line-by-line), bracketed
   paste is working.
4. If not, enable bracketed paste via crossterm's `EnableBracketedPaste`.

## Experiments

### Experiment 1: Check reedline source for bracketed paste default

#### Description

Determine whether reedline enables bracketed paste by default or requires
explicit opt-in.

#### Findings

Reedline has a `BracketedPasteGuard` struct
(`vendor/reedline/src/terminal_extensions/bracketed_paste.rs`) with `enabled`
defaulting to `false`. The builder method `.use_bracketed_paste(true)` must be
called explicitly.

Shannon's `build_editor()` was not calling it — bracketed paste was disabled.

#### Changes

**`src/main.rs`** — added `.use_bracketed_paste(true)` to the reedline builder
in `build_editor()`.

#### Verification

1. `cargo build` succeeds.
2. `cargo test` — all 27 tests pass.

**Result:** Pass

## Conclusion

Bracketed paste was not enabled by default in reedline. Added
`.use_bracketed_paste(true)` to the reedline builder — a one-line fix.
