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

#### Research findings

**Summary of studied repos:**

| Repo     | Stack      | Prompt                             | History                | Compaction                           | Tools                          | Confirmation                         |
| -------- | ---------- | ---------------------------------- | ---------------------- | ------------------------------------ | ------------------------------ | ------------------------------------ |
| Codex    | Rust + TS  | Template files, per-model variants | JSONL on disk          | LLM-driven summary                   | shell, patch, web search, MCP  | 3 modes: suggest/auto-edit/full-auto |
| OpenCode | TypeScript | Dynamic, plugin-hookable           | SQLite via Drizzle ORM | Prune tool output, then LLM summary  | Skills (SKILL.md), agent modes | Agent-mode permissions               |
| Pi       | TypeScript | Static, simple composition         | JSONL on disk          | Prune + LLM summary, manual /compact | read, bash, edit, write        | None (extension-based)               |

**Key patterns across all three:**

1. **Conversation history on disk** — all three persist chat history (JSONL or
   SQLite). None use memory-only.
2. **Two-stage compaction** — prune tool outputs first (cheap), then LLM
   summarization (expensive). Triggered by context overflow.
3. **System prompt = template + context** — all inject shell/cwd/OS info into
   the system prompt alongside static instructions.
4. **Streaming** — all three stream responses. Even for short responses, it
   provides visual feedback that something is happening.
5. **Provider abstraction** — all support multiple providers via a common
   interface (though the abstraction complexity varies hugely).
6. **Tools are optional for MVP** — Pi's simplest mode is just conversation with
   no tools. Codex and OpenCode add tools incrementally.

**Anthropic Messages API (minimal call):**

```
POST https://api.anthropic.com/v1/messages
Headers:
  x-api-key: <key>
  anthropic-version: 2023-06-01
  content-type: application/json

Body:
{
  "model": "claude-sonnet-4-20250514",
  "max_tokens": 1024,
  "system": "You are a shell command translator...",
  "messages": [
    { "role": "user", "content": "list rust files modified today" }
  ]
}

Response:
{
  "content": [{ "type": "text", "text": "fd --extension rs --changed-within 1d" }],
  "stop_reason": "end_turn",
  "usage": { "input_tokens": 42, "output_tokens": 15 }
}
```

That's it for MVP — one HTTP POST, parse the text content.

#### Architecture proposal for shannon

**Answers to all 22 open questions:**

1. **Execute through normal path?** Yes. AI suggests a command, user confirms,
   it goes through `execute_command` with the active shell's config.

2. **Single or multiple commands?** Single command for MVP. The LLM is
   instructed to output exactly one command.

3. **Auto-retry on failure?** No for MVP. User sees the error and can ask again.
   Later: optionally feed the error back to the LLM.

4. **Context sent?** Shell name, cwd, OS (from `std::env::consts::OS`). Enough
   for good command generation.

5. **Directory listing?** No for MVP. Privacy concern and performance cost.
   Defer to later.

6. **Command history in context?** No for MVP. May contain secrets. Defer to
   opt-in later.

7. **Conversational or independent?** Conversational within a session. The AI
   remembers what you asked before so you can say "now sort those by date."

8. **When does conversation reset?** When you exit AI mode (Enter on empty
   line). Each AI mode activation starts fresh. This is simple and predictable.

9. **Chat log storage?** In-memory only for MVP. The conversation resets when
   you exit AI mode, so there's nothing to persist. The actual commands that run
   are recorded in history.db via reedline. Later: persist chat sessions to
   disk.

10. **Compaction?** Not needed for MVP. Conversations are short (a few questions
    per AI mode activation). If someone has a very long session, the LLM will
    hit context limits and we can handle that later.

11. **Per-shell chat history?** No. One conversation per AI mode activation,
    regardless of shell. The shell only affects the system prompt.

12. **Multiple providers?** Use OpenAI-compatible API format for MVP. This
    covers Anthropic (via their Messages API), OpenAI, and local models
    (Ollama). But for true MVP simplicity, start with Anthropic's native
    Messages API only — it's one HTTP POST. Add OpenAI-compatible later.

13. **Minimum config?** Just the API key (in env.sh). Model and provider can
    have sensible defaults:
    ```toml
    [ai]
    provider = "anthropic"
    model = "claude-sonnet-4-20250514"
    ```

14. **Anthropic native or OpenAI-compatible?** Anthropic native for MVP. The API
    is simple (one POST endpoint) and we already have the API key in env.sh.
    OpenAI-compatible is more universal but adds complexity.

15. **Tools for MVP?** None. Just command generation. The LLM receives a
    question, outputs a command. No file reading, no web search.

16. **Future tools?** File read (for context), man page lookup, web search.
    These would use Anthropic's tool_use feature.

17. **Privacy?** MVP sends: system prompt (static), shell name, cwd, and the
    user's question. No history, no directory listing, no file contents. The
    user explicitly opts in by entering AI mode and typing a question.

18. **Opt-in levels?** Not for MVP. Just the minimal context. Later: config
    options for including history, directory listing, etc.

19. **Edit flow?** Yes — pre-fill reedline input with the suggested command. The
    user can edit it, then press Enter to run. This reuses existing
    infrastructure.

20. **API errors?** Print the error and return to AI mode prompt. "shannon: AI
    error: <message>". User can try again. Missing API key: print a helpful
    message about setting it in env.sh.

21. **Loading indicator?** Print "Thinking..." while waiting. Simple `eprint!`
    before the API call, cleared when the response arrives.

22. **MVP scope:** Anthropic-only, single question → single command,
    conversational within one AI mode activation, in-memory chat log, no tools,
    no directory listing, no history context. Config: provider + model +
    api_key_env in config.toml. UX: Enter on empty → AI mode, type question →
    see command → Enter/Esc/e.

**Config.toml schema:**

```toml
[ai]
provider = "anthropic"           # only option for MVP
model = "claude-sonnet-4-20250514"  # default model
api_key_env = "ANTHROPIC_API_KEY"   # env var name for the key
```

**System prompt (draft):**

```
You are a shell command translator. The user describes what they want to do
in plain English. You respond with exactly one shell command — nothing else.
No explanation, no markdown, no code fences. Just the command.

The user is in a {shell_name} shell on {os} in the directory {cwd}.

If the request is ambiguous, pick the most common interpretation.
If you truly cannot generate a command, respond with: # unable to generate command
```

**HTTP dependency:** `ureq` (synchronous, minimal) or `reqwest` (async,
heavier). For MVP, `ureq` is simpler — we don't need async for a single blocking
API call. The user waits for the response anyway.

#### Revised decisions: building for the full agent

The MVP is a command translator, but shannon will eventually become a full
coding agent. This affects infrastructure decisions — we should build the
foundation right even if MVP only uses a fraction of it.

**Revised answers:**

- **Q9 (Chat storage):** Disk, not in-memory. Use JSONL files per session in
  `~/.config/shannon/sessions/`. JSONL is the format Codex and Pi converged on —
  append-only, simple, one JSON object per line. Each AI mode activation creates
  a new session file. This is simpler than adding a table to history.db and
  avoids coupling the chat system to reedline.

- **Q12 (Provider abstraction):** Define a `Provider` trait from day one. MVP
  implements `AnthropicProvider` only. The trait handles: send messages, parse
  response, extract command text. Adding OpenAI-compatible later means
  implementing the trait, not refactoring the call site.

- **Q7/Q8 (Conversation model):** Session-aware from the start. Each AI mode
  activation creates a session with a UUID. Messages are appended to the
  session's JSONL file. For MVP, the conversation resets when you exit AI mode
  (new session next time). But the infrastructure supports resuming sessions
  later.

- **System prompt:** Composable from sections, not a static string. A
  `PromptBuilder` that concatenates: base instructions → shell context → tool
  descriptions (empty for MVP) → project context (empty for MVP). Adding tools
  or project context later means adding a section, not rewriting the prompt.

- **API call structure:** Use the full Messages API format with the `tools`
  field (empty array for MVP). This means the response parsing already handles
  `tool_use` content blocks even though none will appear yet.

**What doesn't change:**

- MVP scope: single question → single command, Anthropic only, no tools
- Activation: Enter on empty line
- Confirmation: Enter/Esc/e
- Config: `[ai]` section in config.toml
- HTTP client: `ureq` (synchronous, simple)

**Revised module structure:**

```
src/
├── ai/
│   ├── mod.rs          ← re-exports
│   ├── provider.rs     ← Provider trait + AnthropicProvider
│   ├── prompt.rs       ← PromptBuilder (composable system prompt)
│   ├── session.rs      ← Session (JSONL read/write, message history)
│   └── translate.rs    ← translate_command() — orchestrates the AI call
├── config.rs           ← [ai] section added
└── main.rs             ← AI mode toggle, confirmation UX
```

**Result:** Pass

All three repos analyzed, all 22 questions answered, architecture revised for
full agent trajectory.

#### Conclusion

The research is complete. MVP is a command translator, but the infrastructure is
designed for a full coding agent: Provider trait, disk-backed sessions,
composable prompts, Messages API with tools support. The scope is intentionally
narrow (one API call, no tools) but the foundation scales.

### Experiment 2: End-to-end AI mode MVP

#### Description

Wire up the complete AI mode flow: toggle → question → API call → command →
confirm → execute. Smallest vertical slice that proves the concept. No
streaming, no edit mode, no tools.

#### Library choice: rig-core

Researched four multi-provider Rust LLM crates:

| Crate          | Recent downloads | Providers                                  | Maturity                         |
| -------------- | ---------------- | ------------------------------------------ | -------------------------------- |
| `rig-core`     | 222K             | 20+ (Anthropic, OpenAI, Ollama, Gemini...) | Stable, 6K GitHub stars          |
| `async-openai` | 1.9M             | OpenAI only                                | Very mature, but single-provider |
| `genai`        | 46K              | 10+                                        | Beta (0.6.0-beta)                |
| `llm`          | 17K              | 15+                                        | Smaller community                |

**Decision: `rig-core`** — largest multi-provider crate, first-class Anthropic
support, agent/tool abstractions that align with our full-agent roadmap,
actively maintained. Requires tokio (async), which we'll need for streaming
later anyway.

#### Changes

**`Cargo.toml`** — add dependencies:

```toml
rig-core = { version = "0.33", default-features = false, features = ["reqwest-rustls"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
uuid = { version = "1", features = ["v4"] }
```

**`src/ai/mod.rs`** (new) — re-exports.

**`src/ai/prompt.rs`** (new):

`PromptBuilder` with composable sections:

- `base()` — static instructions ("You are a shell command translator...")
- `context(shell_name, cwd, os)` — runtime context
- `build()` → concatenated string

For MVP, just base + context. Later: tools section, project context, etc.

**`src/ai/session.rs`** (new):

`Session` struct holding conversation messages and writing JSONL to disk:

- `id: String` (UUID v4)
- `messages: Vec<(String, String)>` — (role, content) pairs
- `session_dir: PathBuf`

Methods:

- `Session::new()` — creates session with UUID, ensures session dir exists
- `add_message(&mut self, role, content)` — appends to in-memory list
- `save(&self)` — writes full message list to `{session_dir}/{id}.jsonl`
- `history_for_prompt(&self) -> Vec<rig ChatMessage>` — converts to rig's
  message format for the API call

JSONL format: `{"role":"user","content":"..."}\n` per line.

**`src/ai/translate.rs`** (new):

`translate_command(config, session, shell_name, cwd, question) -> Result<String>`:

1. Add user message to session
2. Build system prompt via PromptBuilder
3. Create rig-core Anthropic client from API key
4. Build a chat completion request with system prompt + session messages
5. Send request, get response text
6. Strip any markdown code fences or explanation from response
7. Add assistant response to session
8. Save session to disk
9. Return the command text

Uses `tokio::runtime::Runtime::new().block_on()` to run the async rig-core call
from our synchronous main loop. This is fine for MVP — one blocking call while
the user waits.

**`src/config.rs`** — add `[ai]` section:

```rust
#[derive(Deserialize, Default)]
pub struct AiConfig {
    pub provider: Option<String>,      // default: "anthropic"
    pub model: Option<String>,         // default: "claude-sonnet-4-20250514"
    pub api_key_env: Option<String>,   // default: "ANTHROPIC_API_KEY"
}
```

Add `pub ai: AiConfig` to `ShannonConfig` (with `#[serde(default)]`).

**`src/main.rs`** — AI mode integration:

Add `ai_mode: bool` state variable (starts false). Add
`ai_session: Option<Session>` (None when not in AI mode).

In the main loop, when `Signal::Success(line)` and line is empty/whitespace:

- If not in AI mode → set `ai_mode = true`, create new Session, continue
- If in AI mode → set `ai_mode = false`, drop session, continue

When in AI mode and line is non-empty:

1. Print "Thinking..."
2. Call `translate_command(...)` with the line as the question
3. If error → print error, continue in AI mode
4. Print the suggested command: `→ {command}`
5. Print `[Enter] run  [Esc] cancel`
6. Read a single key from stdin (crossterm raw mode):
   - Enter → run the command via `execute_command` with active shell
   - Esc → cancel, return to AI mode prompt
7. After execution, exit AI mode (back to normal)

**`src/prompt.rs`** — update prompt display:

When `ai_mode` is true, render `[nu:ai]` instead of `[nu]`. Add `ai_mode:
bool`
field to `ShannonPrompt`.

**`src/lib.rs`** — add `pub mod ai;`

#### What's deferred

- Streaming (print tokens as they arrive)
- Edit mode (`e` to edit the suggested command)
- Multiple providers (rig-core supports them, we just wire one for MVP)
- Tools
- Compaction
- Config validation (missing API key detected at call time, not startup)

#### Verification

1. `cargo build` succeeds.
2. `cargo test` passes (no regressions — AI tests are manual for now).
3. Run shannon, press Enter on empty line → prompt changes to `[nu:ai]`.
4. Type "list all files in this directory" → see `ls` (or similar).
5. Press Enter → command executes, output appears, back to normal mode.
6. Press Esc → cancel, stay in AI mode.
7. Press Enter on empty in AI mode → back to normal mode.
8. Ask a follow-up ("now show only the .rs files") → AI remembers context.
9. Missing API key → helpful error message.
10. `~/.config/shannon/sessions/` contains a JSONL file after a session.

**Result:** Pass

AI mode is working end-to-end. Enter on empty line toggles AI mode, prompt
shows `[nu:ai]`, LLM generates commands via rig-core + Anthropic, user
confirms with Enter/Esc, command executes through the active shell. 76 tests
pass (56 unit + 20 integration). Sessions saved as JSONL to disk.

Fixes applied during implementation:
- Syntax highlighting disabled in AI mode (plain text for natural language)
- Nushell wrapper restored to capture+print pattern (commands like `pwd`
  need explicit `| print` in nushell `-c` mode)
- Editor rebuilt when toggling AI mode (to switch highlighter)

Known issue: vim doesn't open in nushell mode (pre-existing nushell wrapper
issue with `try { }` capturing stdout from interactive programs — separate
from AI mode).

#### Conclusion

AI mode MVP is working. The architecture is solid: Provider trait via
rig-core, composable system prompt, disk-backed JSONL sessions, confirmation
UX. The foundation supports adding streaming, tools, multiple providers, and
full agent capabilities incrementally.
