+++
status = "open"
opened = "2026-03-21"
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
