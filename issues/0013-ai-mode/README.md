+++
status = "open"
opened = "2026-03-22"
+++

# Issue 13: AI mode

## Goal

Add an AI mode to shannon that translates natural language into shell commands.
The AI is a layer on top of the active shell — it generates commands in the
syntax of whichever shell you're currently using, and you confirm before
execution.

## Background

Shannon's README describes it as "an AI-first shell with seamless access to
bash, nushell, and any other shell." The poly-shell is working. The AI mode
is the other half of the vision.

### The idea

You're in nushell mode. You don't remember the exact syntax for something.
Instead of switching to a browser, you toggle AI mode and describe what you
want in plain English:

```
[nu] ~/project >            ← press Enter on empty line
[nu:ai] ~/project > find all rust files modified in the last day
  → fd --extension rs --changed-within 1d
  [Enter] run  [Esc] cancel  [e] edit
[nu] ~/project >            ← back to normal after execution
```

The AI knows you're in nushell, so it generates nushell-compatible commands.
In bash mode, the same question might produce different syntax.

### Activation: Enter on empty line

Press Enter with no input to toggle AI mode. Press Enter again with no input
to toggle back. This is the simplest possible activation — no new keybindings,
no prefix characters, no conflicts.

- **Normal mode → AI mode**: Enter on empty line
- **AI mode → Normal mode**: Enter on empty line, or Esc
- After a command executes, automatically returns to normal mode

The prompt changes to `[nu:ai]` to indicate AI mode is active.

### AI as a layer, not a shell

The AI is not a separate shell in the toggle list. It's a mode that works
with whatever shell is active. This means:

- In nushell mode, AI generates nushell commands
- In bash mode, AI generates bash commands
- In fish mode, AI generates fish commands
- The generated command runs through the active shell's wrapper, so env
  capture works exactly as normal
- History records the actual command that ran, not the natural language prompt

### LLM integration

The AI mode needs to call an LLM. Design considerations:

**Provider flexibility**: Support multiple LLM providers — Anthropic (Claude),
OpenAI, local models (Ollama). The provider and API key should be configurable
in `config.toml`:

```toml
[ai]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
api_key_env = "ANTHROPIC_API_KEY"
```

Or for local models:

```toml
[ai]
provider = "openai"
model = "llama3"
endpoint = "http://localhost:11434/v1"
api_key_env = "OLLAMA_API_KEY"
```

Using an OpenAI-compatible API format covers most providers.

**Context sent to the LLM**: The prompt should include:

- The active shell name and version
- The current working directory
- Recent command history (last 5-10 commands)
- The user's natural language request
- A system prompt instructing the LLM to output a single shell command

**Streaming vs non-streaming**: For a shell command translation, the response
is short (one line usually). Non-streaming is simpler and sufficient. The user
sees a brief pause, then the proposed command.

### Confirmation UX

After the AI generates a command, the user must confirm before execution:

```
[nu:ai] ~/project > list all docker containers
  → docker ps -a
  [Enter] run  [Esc] cancel  [e] edit
```

- **Enter**: execute the command in the active shell
- **Esc**: cancel, return to AI mode prompt
- **e**: open the command in the line editor for manual editing, then
  Enter to run

The confirmation step is critical — no AI-generated command should ever run
without the user's explicit approval.

### What this is NOT

- Not a chat interface — single question → single command
- Not a code generator — generates shell commands, not scripts
- Not autonomous — always requires confirmation
- Not a replacement for knowing your shell — it's a convenience for when
  you forget syntax

### Dependencies

- HTTP client for API calls (reqwest or ureq)
- JSON serialization (already have serde_json)
- The API key must be available in the environment (set via env.sh)
