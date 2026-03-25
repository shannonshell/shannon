+++
status = "closed"
opened = "2026-03-25"
closed = "2026-03-25"
+++

# Issue 30: Replace AI mode with a chat shell engine

## Goal

Replace the current AI command-generation mode with a simple chat shell. The AI
is just another shell in the Shift+Tab rotation — type a message, get a
response. No command generation, no confirmation flow, no execution. Just
conversation.

## Background

The current AI mode (`/ai on`) translates natural language to shell commands,
shows a confirmation prompt, and executes on Enter. This is overly specific. A
general-purpose chat interface is more useful:

- Ask questions about code, errors, or systems
- Get explanations without executing anything
- Conversational follow-ups with context
- Later: add tools (run commands, read files) as extensions

### Design

AI is a `ShellEngine` called "ai" (or "chat"). It appears in the Shift+Tab
rotation alongside "nu" and "brush". When active:

- The prompt shows `[ai]` (like `[nu]` or `[brush]`)
- User types a message, presses Enter
- Shannon sends it to the LLM with context (cwd, OS, previous messages)
- The response is printed to the terminal
- Conversation history persists within the session

The `AiEngine` implements `ShellEngine`:

- `inject_state()` — captures cwd and env for context
- `execute(message)` — sends to LLM, prints response, returns state unchanged
  (no env/cwd changes from a chat message)

### What gets removed

- `/ai` meta-command (AI mode toggle)
- `read_confirmation()` and the Enter/Esc confirmation UI
- `translate_command()` and the command-generation prompt
- The `ai_mode` boolean and `ai_session` in the REPL loop
- The special AI mode code paths in the REPL (the entire `if ai_mode { ... }`)
- AI badge in the prompt (replaced by just the `[ai]` shell name)

### What stays

- `src/ai/session.rs` — conversation session (messages list), reused for chat
- `src/ai/provider.rs` — LLM provider setup (rig-core), reused
- `AiConfig` in config.rs — provider, model, api_key_env settings
- AI prompt builder (adapted for chat instead of command generation)

### What changes

- New `src/ai_engine.rs` — `AiEngine` implementing `ShellEngine`
- `src/repl.rs` — remove all AI mode branching, AI is just another shell
- `src/main.rs` — add `AiEngine` to the shell list
- `src/prompt.rs` — remove `ai_mode` flag and AI badge (the `[ai]` shell name
  serves the same purpose)
- Default shell rotation: `["nu", "brush", "ai"]`

### Future extensions

Once the chat shell works, tools can be added later:

- `run_command(shell, command)` — execute in a real shell
- `read_file(path)` — read a file
- `search(pattern)` — grep the codebase

These are future issues, not part of this one.

## Experiments

### Experiment 1: AiEngine as a chat shell

#### Description

Create `AiEngine` implementing `ShellEngine`. Add it to the shell rotation.
Remove the old AI mode code paths from the REPL. Adapt the existing AI
infrastructure (session, provider, prompt) for chat instead of command
translation.

#### Changes

**`shannon/src/ai_engine.rs`** (new file):

`AiEngine` struct holding `AiConfig`, `Session`, and captured state (cwd, OS).
Implements `ShellEngine`:

- `inject_state()` — store cwd for context
- `execute(message)` — send to LLM, print response to stderr, return state
  unchanged

Uses the existing `Session` for conversation history. Uses the same
rig-core/Anthropic provider as before. System prompt changed from "command
translator" to "helpful assistant with shell context."

**`shannon/src/ai/prompt.rs`**:

Replace `base()` (command translator instructions) with a chat-oriented system
prompt: "You are a helpful assistant. The user is working in a shell. Answer
questions, explain commands, help with code."

**`shannon/src/ai/translate.rs`**:

Rename to `src/ai/chat.rs`. Replace `translate_command()` with `chat()` that
returns the full response text (no code fence stripping, no command extraction).

**`shannon/src/repl.rs`**:

- Remove `ai_mode` boolean and `ai_session` variable
- Remove `use crate::ai::session::Session`
- Remove `use crate::ai::translate::translate_command`
- Remove the entire `if ai_mode { ... }` branch in the REPL loop
- Remove `/ai` from `handle_meta_command`
- Remove `read_confirmation()` function
- Remove AI-related imports (crossterm event, KeyCode for confirmation)
- The REPL just calls `run_command()` for every shell including AI

**`shannon/src/prompt.rs`**:

- Remove `ai_mode` field from `ShannonPrompt`
- Remove `ai_badge_style` field
- Remove the AI badge rendering in `render_prompt_left`
- The `[ai]` shell name in the prompt serves the same purpose

**`shannon/src/main.rs`**:

- Add `AiEngine` to the shell list:
  `ShellSlot { name: "ai", highlighter: None, engine: Box::new(AiEngine::new(config.ai)) }`
- Default rotation: `["nu", "brush", "ai"]`

**`shannon/src/config.rs`**:

- Add "ai" to `BUILTIN_SHELLS` with `highlighter: None`
- Remove `/ai` from any remaining references

**`shannon/src/lib.rs`**:

- Add `pub mod ai_engine;`

#### Verification

1. `cargo build` succeeds.
2. `cargo test` — all tests pass.
3. Shift+Tab cycles through nu → brush → ai → nu.
4. In `[ai]` shell: type a question, get a response printed.
5. Follow-up questions remember context.
6. `/switch ai` works.
7. `/switch nu` returns to nushell with state intact.
8. Old `/ai` command no longer exists.

**Result:** Pass

All verification steps confirmed. 63 tests pass. Chat works with
conversational follow-ups, context awareness (cwd, OS), and markdown
formatting. API key is read from the injected shell state (set via env.sh).

One fix during implementation: `AiEngine` initially read the API key from
`std::env::var()`, but the key is set in shannon's shell state (via env.sh),
not the process environment. Fixed to read from `last_state.env` first.

#### Conclusion

AI is now a shell engine. The REPL has no AI-specific code — it's just another
`run_command()` call. The old AI mode (`/ai`, confirmation UI, command
translation) is gone.

## Conclusion

AI mode replaced with an AI chat shell. Shift+Tab cycles through nu, brush,
and ai. The REPL is shell-agnostic — every shell (including AI) goes through
the same `ShellEngine` trait. Conversation history persists within the session.
