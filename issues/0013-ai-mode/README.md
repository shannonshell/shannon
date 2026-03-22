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
bash, nushell, and any other shell." The poly-shell is working. The AI mode is
the other half of the vision.

### The idea

You're in nushell mode. You don't remember the exact syntax for something.
Instead of switching to a browser, you toggle AI mode and describe what you want
in plain English:

```
[nu] ~/project >            ← press Enter on empty line
[nu:ai] ~/project > find all rust files modified in the last day
  → fd --extension rs --changed-within 1d
  [Enter] run  [Esc] cancel  [e] edit
[nu] ~/project >            ← back to normal after execution
```

The AI knows you're in nushell, so it generates nushell-compatible commands. In
bash mode, the same question might produce different syntax.

### Activation: Enter on empty line

Press Enter with no input to toggle AI mode. Press Enter again with no input to
toggle back. This is the simplest possible activation — no new keybindings, no
prefix characters, no conflicts.

- **Normal mode → AI mode**: Enter on empty line
- **AI mode → Normal mode**: Enter on empty line, or Esc
- After a command executes, automatically returns to normal mode

The prompt changes to `[nu:ai]` to indicate AI mode is active.

### AI as a layer, not a shell

The AI is not a separate shell in the toggle list. It's a mode that works with
whatever shell is active. This means:

- In nushell mode, AI generates nushell commands
- In bash mode, AI generates bash commands
- In fish mode, AI generates fish commands
- The generated command runs through the active shell's wrapper, so env capture
  works exactly as normal
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

**Streaming vs non-streaming**: For a shell command translation, the response is
short (one line usually). Non-streaming is simpler and sufficient. The user sees
a brief pause, then the proposed command.

### Confirmation UX

After the AI generates a command, the user must confirm before execution:

```
[nu:ai] ~/project > list all docker containers
  → docker ps -a
  [Enter] run  [Esc] cancel  [e] edit
```

- **Enter**: execute the command in the active shell
- **Esc**: cancel, return to AI mode prompt
- **e**: open the command in the line editor for manual editing, then Enter to
  run

The confirmation step is critical — no AI-generated command should ever run
without the user's explicit approval.

### What this is NOT

- Not a chat interface — single question → single command
- Not a code generator — generates shell commands, not scripts
- Not autonomous — always requires confirmation
- Not a replacement for knowing your shell — it's a convenience for when you
  forget syntax

### Dependencies

- HTTP client for API calls (reqwest or ureq)
- JSON serialization (already have serde_json)
- The API key must be available in the environment (set via env.sh)

### Open questions

**Execution model:**

1. When the AI suggests a command and the user confirms, does it go through the
   normal `execute_command` path? (Assumed yes.)
2. Single command only, or can the AI suggest multiple commands?
3. If a command fails, does the AI see the error and suggest a fix, or does the
   user ask again manually?

**System prompt:**

4. What context do we send? Shell name, cwd, OS — what else?
5. Do we include directory listing of cwd? Helps quality but has
   privacy/performance cost.
6. Do we include recent command history? Useful context but may contain secrets.

**Conversation:**

7. Is each question independent, or is there conversational flow within a
   session?
8. If conversational, when does the conversation reset? Exiting AI mode?
   Switching shells? Closing the terminal?
9. Where is the chat log stored? Memory only? Disk? Can we reuse history.db?

**Chat management:**

10. What happens when the conversation gets long? Compaction strategy?
11. Does each shell have a different chat history, or is it per-session?

**Provider config:**

12. How do we handle multiple providers? User picks provider + model in
    config.toml.
13. What's the minimum config needed? Just an API key?
14. Do we use the Anthropic API directly, or an OpenAI-compatible endpoint that
    covers most providers?

**Tools:**

15. What tools does the AI need for MVP? Probably none — just command
    generation.
16. What tools would be useful later? File reading, web search, man page lookup?

**Privacy:**

17. What information does the AI have access to? Cwd and shell name are safe.
    Shell history may contain secrets. Directory listings may reveal project
    structure.
18. Should there be opt-in levels? Minimal (shell + cwd only) vs full (history +
    directory listing)?

**UX details:**

19. The "edit" flow — when user presses "e", do we pre-fill reedline input with
    the suggested command?
20. What happens when the API is down, the key is missing, or the call times
    out?
21. Should we show a spinner/loading indicator while waiting for the LLM?

**MVP scope:**

22. What's the absolute minimum that's useful? Proposed: single provider
    (Anthropic), single question → single command, no conversation memory, no
    tools, no directory listing. Send question + shell name + cwd to Claude, get
    a command back, confirm and run.

## Experiments

### Experiment 1: Research AI agent architectures

#### Description

Study the vendored AI agent repos (codex, opencode, pi) to understand how
existing solutions handle the questions above. The goal is to make informed
architecture decisions before writing any AI code.

#### Research tasks

**1. Study OpenAI Codex CLI (`vendor/codex/`):**

- How does it structure the LLM API call? What's in the system prompt?
- How does it handle provider configuration?
- Does it support conversation history? Where is it stored?
- What tools does it provide to the LLM?
- How does it handle command confirmation/approval?
- What context does it send (cwd, env, file listings, etc.)?
- How does it handle errors, timeouts, and rate limits?

**2. Study OpenCode (`vendor/opencode/`):**

- Same questions as above.
- What's the tech stack? (Go? Rust? TypeScript?)
- How does it handle chat compaction for long conversations?
- Does it have a privacy model or access controls?

**3. Study Pi (`vendor/pi/`):**

- Same questions as above.
- What makes it different from codex/opencode?
- Any interesting architectural patterns we should adopt?

**4. Study the Anthropic Messages API:**

- What's the minimal API call to get a command from Claude?
- How does tool use work in the Messages API?
- What's the request/response format?
- How does streaming work (even if we don't use it for MVP)?
- What's the token limit and pricing consideration?

**5. Answer architecture questions for shannon:**

Based on the research, propose answers to all open questions above. Document the
reasoning. The output should be a clear architecture proposal covering:

- API call structure (endpoint, headers, request body)
- System prompt design
- Config.toml schema for the `[ai]` section
- Chat storage decision (memory vs disk vs history.db)
- Conversation model (independent questions vs conversational)
- MVP scope (exactly what ships in experiment 2)
- Privacy model
- Error handling strategy

#### Verification

1. All three repos are analyzed with findings documented.
2. Every open question above has a proposed answer with reasoning.
3. The architecture proposal is concrete enough to implement in experiment 2.
4. MVP scope is clearly defined — what's in, what's deferred.
