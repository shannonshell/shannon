+++
status = "open"
opened = "2026-03-25"
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
