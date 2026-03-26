+++
status = "open"
opened = "2026-03-26"
+++

# Issue 32: Rearchitect shannon as a fork of the nushell binary

## Goal

Research whether shannon should be restructured from "a shell that wraps
nushell" to "nushell, enhanced with brush and AI." Fork nushell's binary (not
just its library), and add mode switching to brush and AI from within nushell's
native REPL.

## Background

### Current architecture

Shannon has its own REPL (`src/repl.rs`) that uses reedline directly. It embeds
nushell and brush via their library APIs (`eval_source` and `run_string`). This
gives us command evaluation but loses everything the nushell binary provides:
terminal ownership, process groups, job control, signal handling, native
multiline editing, plugins, and more.

We've already forked nushell (`shannonshell/shannon_nushell`), brush
(`shannonshell/shannon_brush`), and reedline (`shannonshell/shannon_reedline`)
as submodules with renamed crates on crates.io.

### The problem

Embedding nushell via `eval_source` loses critical functionality:

- **Job control** — Ctrl+Z doesn't work. Nushell's job control requires terminal
  ownership and process group management that only the nushell binary sets up
  (via `terminal.rs` and `ForegroundChild`).
- **Signal handling** — We've worked around SIGINT with signal-hook, but the
  solution is fragile (double-registration workaround).
- **Multiline editing** — Nushell's REPL has proper multiline support with
  validation. Our REPL doesn't.
- **Completions** — Nushell has context-aware completions for its own commands.
  We use fish completions which don't know nushell syntax.
- **Plugins** — Nushell's plugin system doesn't work through `eval_source`.

### Proposed architecture

Shannon IS nushell — fork the nushell binary as shannon's entry point. Brush and
AI become modes within nushell's native REPL:

```
shannon (= modified nushell binary)
├── [nu] mode   — nushell's native REPL (default)
├── [brush] mode — commands routed to BrushEngine
└── [ai] mode    — messages routed to AiEngine
```

Shift+Tab switches modes. The prompt changes. In nushell mode, everything works
exactly as standalone nushell. In brush mode, the command goes through
`BrushEngine`. In AI mode, the message goes through `AiEngine`.

### What we gain

- Full job control (Ctrl+Z, fg, bg, jobs)
- Proper process groups and terminal management
- Nushell's native multiline editing and validation
- Nushell's context-aware completions
- Nushell's plugin system
- Nushell's configuration system (`config.nu`, `env.nu`)
- All signal handling done correctly by nushell
- Less code to maintain — nushell's REPL replaces ours

### What we lose / change

- Shannon's custom REPL (`src/repl.rs`) — replaced by nushell's
- Shannon's custom completer — replaced by nushell's (better)
- Shannon's custom highlighter — replaced by nushell's (better)
- Shannon's `config.toml` — may need to integrate with nushell's config
- Independence from nushell — deeper coupling to nushell's internals
- Shannon's current reedline keybinding setup — needs to be done via nushell's
  keybinding system instead

### Research questions

1. **Where is nushell's REPL loop?** Can it be modified to dispatch commands to
   different engines based on the active mode?
2. **How does nushell's keybinding system work?** Can we add Shift+Tab to switch
   modes without modifying reedline?
3. **How does nushell handle `eval_source` vs its REPL?** What's the difference
   between the two paths? What does the REPL do that `eval_source` doesn't?
4. **Can we keep shannon as a separate binary** that depends on nushell's
   crates, or do we need to literally fork nushell's `main.rs`?
5. **How does nushell's config system work?** Can we extend it with
   shannon-specific settings (brush, AI, toggle)?
6. **What's the migration path?** Can we do this incrementally, or is it a full
   rewrite?
7. **How do we integrate brush?** When in brush mode, where does the command go?
   Does it bypass nushell's parser entirely and route to `BrushEngine`? How does
   brush receive the raw command string before nushell tries to parse it?
8. **How do we integrate AI?** Same question — AI mode receives plain English,
   not nushell syntax. How do we intercept the input before nushell's parser?
9. **How do we handle Shift+Tab?** Nushell uses reedline keybindings. Can we add
   a custom keybinding that triggers a mode switch without modifying reedline?
   Does nushell's `ExecuteHostCommand` mechanism work for this?
10. **Syntax highlighting per mode** — Nushell highlights nushell syntax. When
    in brush mode, we need bash highlighting. When in AI mode, no highlighting.
    Can we swap the highlighter dynamically? Does nushell rebuild the editor on
    mode switch?
11. **Completions per mode** — Nushell has nushell-aware completions. Brush mode
    needs bash/file completions. AI mode needs no completions (or different
    ones). Can we swap the completer dynamically?
12. **Prompt per mode** — The prompt needs to show `[nu]`, `[brush]`, or `[ai]`.
    Can we change nushell's prompt dynamically from within the REPL loop?
13. **How do we support env.sh?** Shannon currently runs a bash script
    (`env.sh`) at startup to load PATH, API keys, and other env vars. This is
    critical — tutorials and AI always give instructions as "add this to your
    .bashrc." Shannon's `env.sh` lets users follow those instructions directly.
    Nushell uses `env.nu` (nushell syntax) instead. How do we preserve
    bash-based env loading in a nushell-based architecture? Options: run
    `env.sh` via brush at startup and inject the result into nushell's env, or
    source `.bashrc` via brush and propagate.

## Experiments

### Experiment 1: Research nushell internals

#### Description

Read the vendored nushell source code to answer all 13 research questions.

#### Findings

**1. Where is nushell's REPL loop?**

`nu-cli/src/repl.rs` — `evaluate_repl()` (line 71) sets up state, then calls
`loop_iteration()` in a loop (lines 188-247). Each iteration:

- Merges env from previous iteration
- Resets signals
- Evaluates hooks (pre_prompt, env_change, pre_execution)
- Sets up keybindings via `setup_keybindings()`
- Updates prompt via `update_prompt()`
- Calls `line_editor.read_line()` for input
- Parses with `parse_operation()` (auto-cd detection, etc.)
- Evaluates with `eval_source()` via `do_run_cmd()`
- Evaluates post-execution hooks

Before the loop, `run_repl()` in `src/run.rs` calls `setup_config()` to load
env.nu and config.nu. The binary's `main.rs` handles terminal acquisition,
process groups, and signal handlers.

**2. How does nushell's keybinding system work?**

Keybindings are defined in `$env.config.keybindings` (nushell config). Each
entry has `modifier`, `keycode`, `event`, `mode`, `name`. They're re-parsed
every REPL iteration via `create_keybindings()` in `reedline_config.rs`.

Custom keybindings can use `ExecuteHostCommand` to run arbitrary nushell code.
Shift+Tab can be added via config without modifying reedline:

```nushell
{ name: "switch_mode", modifier: "shift", keycode: "backtab",
  event: { send: "ExecuteHostCommand", cmd: "shannon-switch" },
  mode: ["emacs", "vi_insert", "vi_normal"] }
```

**3. eval_source vs REPL — what's different?**

`eval_source` is pure evaluation: parse → merge delta → eval_block → print. The
REPL adds: terminal ownership, process groups, signal handlers, env merging
between iterations, hooks (pre_prompt, pre_execution, post_execution), shell
integration (OSC sequences), prompt management, keybinding setup, history, and
reedline configuration. All of this is lost when embedding via `eval_source`
alone.

**4. Can shannon be a separate binary?**

Yes. Nushell's `main.rs` does: parse CLI args, init EngineState, set up signals
and terminal, load standard library, and call `run_repl()`. Shannon could
replicate this with modifications — add brush/AI engines, custom commands, and
mode switching. The heavy lifting is in `nu-cli`, which is a library crate.

**5. How does nushell's config system work?**

Config files load in order: default env → env.nu → config.nu → login.nu. The
`Config` struct in `nu-protocol` has fields for colors, completions,
keybindings, hooks, menus, etc. Unknown config fields are rejected. Plugin
configs use `$env.config.plugins.<name>` — this is the extensibility point.
Shannon-specific settings could live there or in a separate config file.

**6. Migration path**

Incremental. Shannon can start as a separate binary that calls nushell's
`evaluate_repl()` with a modified EngineState. The binary adds: brush engine, AI
engine, env.sh loading, mode switching commands. The nushell fork is already a
submodule. The key modification is to `loop_iteration()` — add a mode check
before evaluation to dispatch to brush or AI when active.

**7. How do we integrate brush?**

When in brush mode, intercept the command AFTER reedline returns it but BEFORE
nushell parses it. In `loop_iteration()`, after `read_line()` returns
`Signal::Success(line)`, check the active mode. If brush, send the line to
`BrushEngine` instead of `parse_operation()` / `do_run_cmd()`. The raw string
bypasses nushell's parser entirely.

**8. How do we integrate AI?**

Same mechanism as brush. When in AI mode, the raw input string goes to
`AiEngine` instead of nushell's parser. The response is printed directly. No
parsing, no evaluation by nushell.

**9. How does ExecuteHostCommand work?**

Keybindings can specify `{ send: "ExecuteHostCommand", cmd: "..." }`. The
command string is executed as nushell code when the key is pressed. This is how
nushell implements features like fzf history search. Shannon can use it for mode
switching — the command sets `$env.SHANNON_MODE` and the REPL loop checks this
variable each iteration.

**10. Syntax highlighting per mode**

The highlighter is recreated EVERY REPL iteration — `NuHighlighter` is
constructed fresh in `loop_iteration()` (line 400). It's a
`Box<dyn
Highlighter>`. In brush mode, substitute a bash highlighter. In AI mode,
substitute a no-op highlighter. The swap happens naturally because the
highlighter is rebuilt each iteration.

**11. Completions per mode**

Same as highlighting — `NuCompleter` is recreated every iteration (line 408).
It's a `Box<dyn ReedlineCompleter>`. In brush mode, substitute a file/command
completer. In AI mode, substitute a no-op or history completer. The swap is
trivial because completers are rebuilt each iteration.

**12. Prompt per mode**

The prompt is rebuilt every iteration via `update_prompt()`. It evaluates
`$env.PROMPT_COMMAND` closures. Shannon can set the prompt closure to check
`$env.SHANNON_MODE` and display `[nu]`, `[brush]`, or `[ai]` accordingly.
Changes take effect immediately on the next iteration.

**13. How do we support env.sh?**

Nushell loads config in order: default env → env.nu → config.nu. Shannon can
inject a step between default env and env.nu: run env.sh via brush, capture the
resulting env vars, and inject them into the stack via `add_env_var()`. This
happens in `setup_config()` in `config_files.rs`. The injected vars are then
available to env.nu and config.nu.

#### Result

Research complete. All 13 questions answered.

**Key finding: This architecture is viable and can be done incrementally.**

The critical insight: nushell rebuilds the highlighter, completer, and prompt
every single REPL iteration. This means mode switching is naturally supported —
check the mode, return the appropriate highlighter/completer/prompt, done. No
need to rebuild the entire editor or restart reedline.

The mode switch mechanism: a keybinding triggers `ExecuteHostCommand` which sets
`$env.SHANNON_MODE`. The next `loop_iteration()` checks this variable and
dispatches accordingly.

#### Conclusion

Rearchitecting shannon as a nushell fork is feasible. The main modification is
to `loop_iteration()` — add a mode check after `read_line()` to dispatch to
brush or AI instead of nushell's parser. Everything else (highlighter,
completer, prompt, keybindings) is already designed to be dynamic and
per-iteration. The env.sh feature can be preserved by injecting bash env vars
during config loading.

### Experiment 2: Fork the binary vs enhance shannon — which approach?

#### Description

Two options for the rearchitecture:

**Option A: Fork the nu binary.** Copy nushell's `main.rs` and `run.rs` into
shannon, modify `loop_iteration()` in our nushell fork to add mode switching.
Shannon's binary IS the modified nushell binary. We delete shannon's current
`repl.rs` and use nushell's REPL directly.

Pros:

- Get everything nushell has for free (terminal, process groups, job control,
  plugins, multiline, native completions, hooks)
- Less code to maintain — nushell's REPL replaces ours
- Guaranteed compatibility with nushell features

Cons:

- Deep coupling to nushell internals — every upstream update touches our code
- Must modify `loop_iteration()` in the nushell fork (not just use it as lib)
- Shannon becomes a nushell distribution, not an independent shell
- Two REPLs to understand (nushell's is ~1000 lines of complex state)

**Option B: Enhance shannon's existing REPL.** Keep shannon's `repl.rs` but add
the missing pieces that nushell's REPL has: terminal ownership, process groups,
signal setup, env merging. Learn from nushell's code but don't copy it.

Pros:

- Shannon stays independent — lighter coupling to nushell
- We control the REPL complexity
- Easier to understand and modify
- Upstream nushell changes don't break our REPL

Cons:

- Must reimplement terminal/process group/job control ourselves
- May never reach full nushell feature parity
- More code to maintain long-term

#### Method

Compare the two approaches by examining:

1. **How much of nushell's main.rs/run.rs would we actually copy?** Is it a thin
   wrapper we can replicate, or deep integration?
2. **What specifically is missing from shannon's REPL?** List every feature gap
   between our `repl.rs` and nushell's `loop_iteration()`.
3. **How hard are the gaps to fill?** Terminal ownership and process groups are
   the big ones. Are they isolated code we can copy, or deeply woven into
   nushell's REPL?
4. **What breaks when nushell updates?** If we fork `loop_iteration()`, how
   often does it change upstream? What's the merge burden?

#### Findings

**1. How much would we copy?**

- `main.rs` (653 lines): ~200-250 lines of actual logic needed. The rest is
  feature gates, IDE/LSP modes, and plugin setup we can skip initially.
- `run.rs` (221 lines): `run_repl()` is only 40 lines — thin wrapper around
  `evaluate_repl()` from nu-cli.
- `terminal.rs` (148 lines): Highly isolated. Pure terminal/process group setup
  using `nix` crate. Can be extracted as-is.
- `signals.rs` (34 lines): Single function. Trivially isolated.

Total to copy for Option A: ~600 lines of integration code, not 1700.

**2. What's missing from shannon's REPL?**

Shannon's REPL is 295 lines. Nushell's is 1,759 lines. 22 feature gaps found:

Critical gaps:

- Terminal ownership / job control (149 lines, deeply integrated)
- Hook system — pre_prompt, pre_execution, env_change (300+ lines)
- Env merging between iterations (5 lines, isolated)
- Config-driven keybindings (1,300+ lines in reedline_config.rs)
- Stack/variable cleanup (80 lines, deeply integrated)

Medium gaps:

- Multiline validation (28 lines, isolated)
- Transient prompt (140 lines)
- Auto-cd (60 lines)
- Shell integration OSC 133/633 (150 lines)
- Menu system (250+ lines)
- Buffer editor integration (15 lines)
- Panic recovery via catch_unwind (58 lines)
- REPL buffer management (30 lines)

Low priority:

- History metadata, CMD_DURATION_MS, cursor shapes, kitty protocol, mouse

**3. How hard are the gaps to fill?**

Terminal ownership and process groups (`terminal.rs`) are isolated — 148 lines
that can be copied directly. The hard parts are deeply woven into nushell's
REPL: the hook system, config-driven keybindings, and stack management. These
aren't isolated modules — they're interleaved throughout `loop_iteration()`.

If we go with Option B (enhance shannon), we'd need to reimplement these from
scratch. If we go with Option A (fork the binary), we get them for free but must
maintain the fork.

**4. How often does the REPL change?**

- `repl.rs`: 20 commits since Jan 2025 (~1/week). Actively maintained.
- `main.rs`: 47 commits. High activity.
- `terminal.rs`: 2 commits. Stable.
- `signals.rs`: 2 commits. Stable.

The REPL changes frequently for shell integration (OSC sequences), reedline
upgrades, and new features. Forking it means regular merge work.

#### Decision

**Option A: Fork the binary.** The gap analysis makes this clear:

- Shannon's REPL has 22 missing features. Reimplementing them (Option B) means
  writing ~2000+ lines of complex state management that nushell already has.
- The isolated pieces (terminal.rs, signals.rs) are stable and easy to extract,
  but the deeply integrated pieces (hooks, keybindings, stack management) are
  not — they require the full REPL context.
- The maintenance burden of tracking nushell's REPL changes (~1/week) is less
  than the development burden of reimplementing and maintaining 22 features.
- Shannon already deeply depends on nushell (forked crates, renamed packages).
  The additional coupling of forking the binary is marginal.

The approach: modify `loop_iteration()` in our nushell fork to check a mode
variable. When in brush or AI mode, dispatch to the respective engine instead of
nushell's parser. Everything else (terminal, signals, hooks, keybindings,
completions, highlighting, prompt) comes from nushell's REPL for free.

**Result:** Decision made — Option A.

#### Conclusion

Fork the binary. The 22-feature gap between shannon's REPL and nushell's is too
large to reimplement. The maintenance cost of tracking nushell's REPL changes is
lower than the development cost of building all missing features. Next
experiment: implement the fork.

### Experiment 3: Scope the implementation

#### Description

Map out every change needed to make this work, across both repos. The goal: the
nushell binary IS shannon, with brush and AI as switchable modes.

#### Architecture

```
nushell/ (submodule, shannon branch)
├── Cargo.toml          — adds shannon crate as dependency
├── src/main.rs         — builds as "shannon" binary, adds env.sh loading
└── crates/nu-cli/
    └── src/repl.rs     — loop_iteration() checks SHANNON_MODE env var,
                          dispatches to brush/ai engines when active

shannon/ (library crate)
├── src/lib.rs          — exports BrushEngine, AiEngine, ShellEngine trait
├── src/brush_engine.rs — BrushEngine (unchanged)
├── src/ai_engine.rs    — AiEngine (unchanged)
├── src/shell_engine.rs — ShellEngine trait (unchanged)
└── src/executor.rs     — run_startup_script / env.sh loading (unchanged)

brush/ (submodule, unchanged)
reedline/ (submodule, unchanged)
```

Shannon's current `main.rs`, `repl.rs`, `config.rs`, `prompt.rs`,
`highlighter.rs`, `completer.rs` are deleted — nushell provides all of that.

#### Changes needed

**In the nushell fork (`nushell/` submodule):**

1. **`Cargo.toml` (root)** — add `shannonshell` as a dependency so the binary
   can import BrushEngine/AiEngine
2. **`src/main.rs`** — change binary name to "shannon", add env.sh loading
   before config setup, initialize BrushEngine and AiEngine, store them in
   EngineState (or a side channel)
3. **`crates/nu-cli/src/repl.rs`** — in `loop_iteration()`, after `read_line()`
   returns `Signal::Success(line)`:
   - Check `$env.SHANNON_MODE` (or equivalent)
   - If "brush": send line to BrushEngine, skip nushell parse/eval
   - If "ai": send line to AiEngine, skip nushell parse/eval
   - If "nu" (default): normal nushell evaluation
4. **`crates/nu-cli/src/repl.rs`** — in highlighter/completer setup:
   - If brush mode: use a bash tree-sitter highlighter, file completer
   - If ai mode: use no-op highlighter, no completer
   - If nu mode: normal nushell highlighter/completer
5. **Keybinding setup** — add default Shift+Tab binding that cycles
   `$env.SHANNON_MODE` between "nu", "brush", "ai"
6. **Prompt** — modify prompt to show `[nu]`, `[brush]`, or `[ai]` based on
   mode. Could be done via default `PROMPT_COMMAND` that checks the mode var.

**In the shannon crate (`shannon/`):**

7. **Delete** — `main.rs`, `repl.rs`, `config.rs`, `prompt.rs`,
   `highlighter.rs`, `completer.rs`, `completions.rs`, `theme.rs`, `shell.rs`,
   `tree_sitter_nu.rs`
8. **Keep** — `lib.rs`, `brush_engine.rs`, `ai_engine.rs`, `shell_engine.rs`,
   `executor.rs` (for env.sh), `ai/` module
9. **`lib.rs`** — re-export only what the nushell binary needs
10. **`Cargo.toml`** — remove reedline, crossterm, tree-sitter, nu-ansi-term and
    other deps that the REPL used. Keep brush, AI, tokio, rig-core.

**Config integration:**

11. Shannon config (`config.toml`) may be replaced by nushell's config system.
    Or keep env.sh for bash env loading and use nushell's `config.nu` for
    everything else.

#### Scope estimate

- Nushell fork changes: ~200-300 lines modified across 3-4 files
- Shannon crate: mostly deletion (~2000 lines removed), ~50 lines of lib.rs
  changes
- New code: ~100 lines (mode dispatch in loop_iteration, env.sh integration)
- Net effect: shannon shrinks dramatically, nushell fork gains ~200 lines

#### Verification

Scope documented. No code changes in this experiment.

### Experiment 4: Research config.nu integration for shannon settings

#### Description

Shannon's config.toml has three categories of settings:

1. **toggle** — shell rotation order (e.g., `["nu", "brush", "ai"]`)
2. **AI config** — provider, model, api_key_env
3. **theme** — 13 color settings (replaced by nushell's native
   `$env.config.color_config`)

Theme is gone — nushell handles it natively. The remaining settings (toggle and
AI config) need a home in nushell's config system. Research how to store
shannon-specific settings in config.nu or env.nu.

#### Research questions

1. **Can we use `$env` vars?** Set `$env.SHANNON_TOGGLE` and
   `$env.SHANNON_AI_MODEL` in env.nu. Simple, no config struct needed. But env
   vars are strings — toggle is a list. Can nushell env vars hold lists?

2. **Can we use `$env.config.plugins`?** Nushell has a `plugins` field in Config
   that accepts arbitrary key-value pairs. Could we store shannon config there?
   How does reading it back work?

3. **Can we add custom fields to `$env.config`?** Nushell rejects unknown config
   fields. Can our fork relax this to accept a `shannon` section?

4. **Can we use a custom `$env.SHANNON_CONFIG` record?** Set a structured record
   in env.nu:
   ```nushell
   $env.SHANNON_CONFIG = {
       toggle: ["nu", "brush", "ai"]
       ai: {
           provider: "anthropic"
           model: "claude-sonnet-4-20250514"
           api_key_env: "ANTHROPIC_API_KEY"
       }
   }
   ```
   Would this survive across REPL iterations? Can the Rust code read it from the
   Stack?

5. **How does the Rust code read env var values from the Stack?** What types
   does `stack.get_env_var()` return? Can it return lists, records, or only
   strings?

#### Method

Read the nushell source code to answer each question. Focus on:

- `nu-protocol/src/config/` — how Config is parsed, what's rejected
- `nu-protocol/src/engine/stack.rs` — how env vars are stored and typed
- `nu-protocol/src/value/` — what Value types exist (string, list, record)
- How existing nushell features read structured env vars

#### Findings

**1. Can nushell env vars hold non-string types?**

YES. `stack.add_env_var(name, value)` takes a `Value`, not a `String`. The
`Value` enum supports Bool, Int, Float, String, Record, List, and more.
`stack.get_env_var()` returns `Option<&Value>`. Nushell itself uses this —
`$env.NUSHELL_SHELLS` is a list, not a string.

**2. Can we use `$env.config.plugins`?**

YES. The `plugins` field is `HashMap<String, Value>` — it accepts any key-value
pairs. We could do `$env.config.plugins.shannon = { toggle: [...] }`. Reading it
from Rust: access `engine_state.get_config().plugins.get("shannon")`.

**3. Can we add custom fields to `$env.config`?**

NO without forking. Unknown top-level config keys are rejected with
`errors.unknown_option(path, val)` in the match statement. Our fork could add a
`"shannon"` arm to the match, but that's more invasive than necessary.

**4. Can `$env.SHANNON_CONFIG` be a persistent record?**

YES. This is the cleanest approach. Set in env.nu:

```nushell
$env.SHANNON_CONFIG = {
    TOGGLE: ["nu", "brush", "ai"]
    AI_PROVIDER: "anthropic"
    AI_MODEL: "claude-sonnet-4-20250514"
    AI_API_KEY_ENV: "ANTHROPIC_API_KEY"
}
```

Records persist across REPL iterations — env vars are merged via
`Stack::with_changes_from_child()` which copies the full
`HashMap<String, Value>`. The record is cloned to child stacks and survives the
merge.

**5. How does Rust code read structured env vars?**

`Value` has methods: `.as_record()`, `.as_list()`, `.as_str()`, `.as_int()`,
`.into_list()`. Pattern from nushell's own code:

```rust
if let Some(val) = stack.get_env_var(engine_state, "SHANNON_CONFIG") {
    if let Ok(record) = val.as_record() {
        if let Some(toggle) = record.get("TOGGLE") {
            let shells = toggle.as_list()?;
            // iterate shells...
        }
        if let Some(model) = record.get("AI_MODEL") {
            let model_str = model.as_str()?;
        }
    }
}
```

#### Decision

**Use `$env.SHANNON_CONFIG` as a structured record.** This is the best approach:

- No nushell config changes needed (no unknown field rejection)
- Fully typed — lists stay lists, records stay records
- Persists across REPL iterations
- Readable from Rust via `stack.get_env_var()` + `.as_record()`
- Users configure it naturally in env.nu or config.nu
- No separate config file

The user's `~/.config/shannon/env.nu` would include:

```nushell
$env.SHANNON_CONFIG = {
    TOGGLE: ["nu", "brush", "ai"]
    AI_PROVIDER: "anthropic"
    AI_MODEL: "claude-sonnet-4-20250514"
    AI_API_KEY_ENV: "ANTHROPIC_API_KEY"
}
```

Default values are set in shannon's startup code if `$env.SHANNON_CONFIG` is not
defined. The Rust code reads the record from the Stack each REPL iteration.

**Result:** Research complete, decision made.

#### Conclusion

Shannon config lives in `$env.SHANNON_CONFIG` as a nushell record. No
config.toml, no config.nu modifications needed. Set it in env.nu. Rust reads it
via `stack.get_env_var("SHANNON_CONFIG")` → `.as_record()` → navigate fields.
Uppercase naming follows nushell convention for env vars.

### Experiment 5: Research env var conversion between nushell and brush

#### Description

Nushell stores some env vars as typed values (e.g., PATH is a list). External
processes need strings. Nushell has `ENV_CONVERSIONS` — closures that convert
between types and strings. When switching to brush, we need string env vars.
When switching back, we need to convert strings back to nushell types.

Research question: does nushell already have functions we can reuse for this, or
do we need to build our own?

#### Findings

**Nushell already has exactly what we need.**

**Nu → Brush (typed values to strings):**

`nu_engine::env_to_strings(engine_state, stack)` returns
`Result<HashMap<String, String>>`. It:

1. Checks `ENV_CONVERSIONS` for a `to_string` closure — uses it if found
2. Falls back to `value.coerce_string()` for simple types (bool, int, string)
3. Special-cases PATH: joins list elements with the OS path separator
4. Silently skips vars that can't be converted to strings

This is the same function nushell uses when spawning external commands
(`run_external.rs` line 178). It's `pub` and exported from `nu-engine`.

**Brush → Nu (strings back to typed values):**

`nu_engine::env::convert_env_values(engine_state, stack)` applies `from_string`
closures from `ENV_CONVERSIONS` to convert string env vars back to typed values.
It's called during nushell initialization to convert the inherited OS
environment.

**The integration pattern:**

```rust
// Nu → Brush: get all env vars as strings
let env_strings = nu_engine::env_to_strings(engine_state, &stack)?;
let state = ShellState {
    env: env_strings,
    cwd: /* from PWD */,
    last_exit_code: 0,
};
brush_engine.inject_state(&state);

// Brush → Nu: write string env vars back to stack
let new_state = brush_engine.execute(&command);
for (key, value) in &new_state.env {
    stack.add_env_var(key.clone(), Value::string(value, Span::unknown()));
}
// Then trigger from_string conversions for vars that have them
```

The `from_string` conversion happens automatically when nushell's env merging
runs at the top of the next REPL iteration — we don't need to call it manually.

**Result:** Research complete.

#### Conclusion

No custom conversion code needed. `nu_engine::env_to_strings()` gives us all env
vars as strings for brush. Writing strings back to the Stack with
`add_env_var(Value::string(...))` is sufficient — nushell's REPL iteration
handles `from_string` conversions automatically on the next loop.

This confirms the bridge between nushell and brush is straightforward:

- `capture_nu_state` = `env_to_strings()` + read PWD
- `apply_state_to_stack` = write `Value::string()` for each env var +
  `set_cwd()`

### Experiment 6: Copy nu binary into shannon, add mode dispatch

#### Description

Copy ALL nu binary source files (~4,600 lines, 11 files) directly into shannon.
Maintain a complete fork of the nu binary. No picking and choosing — simpler to
keep everything and modify what we need. No visibility issues, no import
problems — everything is in the same crate.

**Copy from `nushell/src/` into `shannon/src/`:**

All 11 files: `main.rs`, `command.rs`, `command_context.rs`, `config_files.rs`,
`experimental_options.rs`, `ide.rs`, `logger.rs`, `run.rs`, `signals.rs`,
`terminal.rs`, `test_bins.rs`.

Shannon's `main.rs` IS nu's `main.rs` with modifications.

**Modifications to the copied code:**

1. `main.rs` — add ModeDispatcher creation, env.sh loading, SHANNON_MODE
   default, change config dir to `~/.config/shannon/`
2. `config_files.rs` — add env.sh loading step before env.nu
3. `run.rs` — pass dispatcher to `evaluate_repl()`
4. `terminal.rs` — change "nushell" error messages to "shannon"
5. `signals.rs` — no changes needed

**What stays in shannon's existing code:**

- `brush_engine.rs`, `ai_engine.rs`, `shell_engine.rs`, `shell.rs`,
  `executor.rs`, `ai/` — unchanged
- `dispatcher.rs` — NEW, implements ModeDispatcher

**What gets deleted:**

- `repl.rs`, `config.rs`, `prompt.rs`, `highlighter.rs`, `completer.rs`,
  `completions.rs`, `theme.rs`, `nushell_engine.rs`
- `tree_sitter_nu/`, `build.rs`, `completions/`, `themes/`

#### Phases

**Phase 1:** Already done — ModeDispatcher trait added to nu-cli fork.

**Phase 2:** Copy nu binary source into `shannon/src/nu_binary/`. Modify
shannon's `main.rs` to call the copied startup code. Verify: builds and starts
as nushell.

**Phase 3:** Add `dispatcher.rs`, wire mode dispatch into the copied `run.rs` →
`evaluate_repl()`. Verify: brush and AI modes work.

**Phase 4:** Keybinding (Shift+Tab) and prompt per mode.

**Phase 5:** Highlighter/completer per mode.

**Phase 6:** env.sh loading, `~/.config/shannon/` config dir.

**Phase 7:** Delete old REPL files, update build system, update docs.

#### Verification (end to end)

1. `cargo build` produces a `shannon` binary
2. Shannon starts, shows nushell prompt
3. Shift+Tab cycles nu → brush → ai
4. Env vars propagate across modes
5. cd propagates across modes
6. Ctrl+C works in all modes
7. Ctrl+Z works in nushell mode
8. env.sh loaded at startup
9. Config dir is `~/.config/shannon/`
10. `cargo test` passes
