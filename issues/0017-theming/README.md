+++
status = "open"
opened = "2026-03-23"
+++

# Issue 17: Theming

## Goal

Make shannon themeable with three layers: ANSI colors by default (inherits
terminal theme), named themes from fish's collection, and individual color
overrides in config.toml. Support automatic light/dark mode switching.

## Background

Shannon currently hardcodes Tokyo Night RGB colors everywhere — syntax
highlighting, prompt, hints, menu. This means:

1. Shannon ignores the terminal's theme completely (RGB colors are absolute).
2. Users with light terminals get unreadable colors.
3. No way to customize colors without editing source code.

### The three-layer approach

**Layer 1: ANSI colors (default, zero config)**

Switch from `Color::Rgb(...)` to named ANSI colors (`Color::Green`,
`Color::Cyan`, etc.) for all syntax highlighting and UI elements. The terminal's
theme controls the actual appearance. This is how most shells work — fish, bash,
zsh all use ANSI colors and let the terminal decide what "green" looks like.

This means shannon looks right in any terminal theme automatically. A user with
Dracula in their terminal gets Dracula colors in shannon. A user with Solarized
Light gets Solarized Light. No configuration needed.

**Layer 2: Named themes (pick from fish's collection)**

```toml
[theme]
name = "catppuccin-mocha"
```

Adopts fish's 25 community-maintained `.theme` files. When a named theme is set,
it overrides the ANSI defaults with specific colors (usually RGB). Fish themes
have `[light]` and `[dark]` sections for dual-mode support.

This layer completely overrides the terminal's palette for shannon's output,
giving the user a consistent look regardless of terminal theme.

**Layer 3: Individual overrides (total control)**

```toml
[theme]
name = "dracula"          # start with dracula
keyword = "#ff79c6"       # override just this color
prompt = "green"          # override the prompt color
```

Any individual color can be overridden on top of a named theme or the ANSI
defaults. This is for users who want precise control.

### Semantic color categories

Shannon's theme maps to these categories:

**Syntax highlighting:**

| Category     | What it colors                | ANSI default |
| ------------ | ----------------------------- | ------------ |
| `keyword`    | if, for, let, export, etc.    | Magenta      |
| `command`    | Command names (ls, git, echo) | Blue         |
| `string`     | Quoted strings                | Green        |
| `number`     | Integer and float literals    | Yellow       |
| `variable`   | $FOO, $env.PATH               | Cyan         |
| `operator`   | Pipes, redirections, &&,      |              |
| `comment`    | # comments                    | DarkGray     |
| `error`      | Syntax errors                 | Red          |
| `foreground` | Default text                  | White        |
| `type`       | Type annotations (nushell)    | Yellow       |

**UI elements:**

| Category           | What it colors                     | ANSI default   |
| ------------------ | ---------------------------------- | -------------- |
| `prompt`           | Shell name and path                | Cyan           |
| `prompt_indicator` | > and ! characters                 | DarkGray / Red |
| `hint`             | Autosuggestion ghost text          | DarkGray       |
| `menu_text`        | Completion menu items              | DarkGray       |
| `menu_selected`    | Selected completion item           | Green reverse  |
| `menu_description` | Completion descriptions            | Yellow         |
| `menu_match`       | Matching characters in completions | Underline      |

### Mapping fish theme variables to shannon categories

| Fish variable                          | Shannon category   |
| -------------------------------------- | ------------------ |
| `fish_color_command`                   | `command`          |
| `fish_color_keyword`                   | `keyword`          |
| `fish_color_quote`                     | `string`           |
| `fish_color_redirection`               | `operator`         |
| `fish_color_comment`                   | `comment`          |
| `fish_color_error`                     | `error`            |
| `fish_color_normal`                    | `foreground`       |
| `fish_color_autosuggestion`            | `hint`             |
| `fish_color_param`                     | `foreground`       |
| `fish_color_option`                    | `operator`         |
| `fish_color_escape`                    | `variable`         |
| `fish_pager_color_completion`          | `menu_text`        |
| `fish_pager_color_selected_background` | `menu_selected`    |
| `fish_pager_color_description`         | `menu_description` |
| `fish_pager_color_prefix`              | `menu_match`       |

### Light/dark mode support

Fish themes have `[light]`, `[dark]`, and `[unknown]` sections. Shannon can
detect the system appearance and pick the right section:

- **macOS**: `defaults read -g AppleInterfaceStyle` (returns "Dark" or error)
- **Terminal query**: `OSC 11` response to detect background luminance
- **Config override**: `[theme] mode = "dark"` to force one mode

**Mid-session switching:** When the system switches from dark to light (or vice
versa), the next prompt detects the change and:

1. Emits `OSC 11` to change the terminal background (if the theme defines one)
2. Rebuilds the editor with the new section's colors
3. Everything from that prompt forward uses the new colors

### Theme file management

Same pattern as completions:

- `scripts/update-themes.sh` — copies `.theme` files from
  `vendor/fish/share/themes/` into `themes/` directory in the repo
- Theme files are checked into git
- A custom `tokyo-night.theme` is created manually (fish doesn't ship it)
- Themes are parsed at build time or startup (they're tiny — KB total)

### Config.toml schema

```toml
[theme]
# Pick a named theme (optional — omit for ANSI defaults)
name = "catppuccin-mocha"

# Auto-detect dark/light mode (default: true)
auto = true

# Force a mode (overrides auto-detection)
# mode = "dark"

# Individual color overrides (optional — override any category)
# keyword = "#ff79c6"
# command = "blue"
# string = "#50fa7b"
# prompt = "green"
# hint = "#6272a4"
```

### Color value format

Support the same formats as fish:

- Named ANSI: `"red"`, `"green"`, `"cyan"`, `"magenta"`, etc.
- Bright variants: `"brred"`, `"brgreen"`, `"brcyan"`, etc.
- Hex RGB: `"#FF79C6"` or `"FF79C6"`
- Modifiers: `"green --bold"`, `"cyan --italic"`

### What needs to change

| Component              | Currently               | After                    |
| ---------------------- | ----------------------- | ------------------------ |
| `src/highlighter.rs`   | Hardcoded RGB constants | Reads from theme         |
| `src/repl.rs` (hinter) | Hardcoded `#565f89`     | Reads from theme         |
| `src/repl.rs` (menu)   | Reedline defaults       | Reads from theme         |
| `src/prompt.rs`        | Hardcoded Cyan          | Reads from theme         |
| `src/config.rs`        | No `[theme]` section    | Parses theme config      |
| `build.rs` or startup  | N/A                     | Loads/parses theme files |

## Experiments

### Experiment 1: Switch to ANSI defaults and add theme infrastructure

#### Description

Two changes in one experiment:

1. **Switch all hardcoded RGB colors to ANSI colors.** This immediately
   fixes shannon for users with non-Tokyo-Night terminals. Zero config,
   inherits terminal theme.

2. **Add a `Theme` struct and `[theme]` config section.** This is the
   infrastructure for layers 2 and 3. The theme struct holds all semantic
   colors and is passed to the highlighter, prompt, hinter, and menu.

Defer named theme files (fish `.theme` parsing) and light/dark auto-detection
to later experiments. This experiment establishes the foundation.

#### Changes

**`src/theme.rs`** (new module):

```rust
pub struct Theme {
    // Syntax highlighting
    pub keyword: Style,
    pub command: Style,
    pub string: Style,
    pub number: Style,
    pub variable: Style,
    pub operator: Style,
    pub comment: Style,
    pub error: Style,
    pub foreground: Style,
    pub type_: Style,

    // UI
    pub prompt: Color,
    pub prompt_indicator: Color,
    pub prompt_error: Color,
    pub hint: Style,
    pub menu_text: Style,
    pub menu_selected: Style,
    pub menu_description: Style,
    pub menu_match: Style,
}
```

`Theme::default()` — returns ANSI color defaults:
- keyword → Magenta bold
- command → Blue
- string → Green
- number → Yellow
- variable → Cyan
- operator → Cyan
- comment → DarkGray
- error → Red
- foreground → White
- prompt → Cyan
- hint → DarkGray italic

`Theme::from_config(config: &ThemeConfig)` — applies overrides from
config.toml on top of defaults. Parses color strings ("green", "#FF79C6",
"cyan --bold") into `Style` objects.

`parse_color(s: &str) -> Style` — parses a color string:
- Named: "red", "green", "cyan", "magenta", "white", "brred", etc.
- Hex: "#FF79C6" or "FF79C6"
- Modifiers: "--bold", "--italic", "--underline" appended
- Example: "green --bold" → `Color::Green.bold()`

**`src/config.rs`** — add `[theme]` section:

```rust
#[derive(Deserialize, Default)]
pub struct ThemeConfig {
    pub name: Option<String>,     // for future named themes
    pub keyword: Option<String>,
    pub command: Option<String>,
    pub string: Option<String>,
    pub number: Option<String>,
    pub variable: Option<String>,
    pub operator: Option<String>,
    pub comment: Option<String>,
    pub error: Option<String>,
    pub foreground: Option<String>,
    pub type_: Option<String>,
    pub prompt: Option<String>,
    pub hint: Option<String>,
    // ... menu colors
}
```

Add `pub theme: ThemeConfig` to `ShannonConfig`.

**`src/highlighter.rs`** — accept `Theme` instead of hardcoded colors:

Replace the `const` color values with fields read from the theme. Change
`TreeSitterHighlighter::new` to accept `&Theme`:

```rust
pub fn new(highlighter: Option<&str>, theme: &Theme) -> Self
```

Store the theme's colors as fields on the struct. The `style_for_node` and
color methods use these fields instead of constants.

**`src/repl.rs`** — pass theme to components:

- Create `Theme` at the start of `run()` from config
- Pass `&theme` to `build_editor`
- In `build_editor`:
  - `TreeSitterHighlighter::new(highlighter, &theme)`
  - `DefaultHinter::default().with_style(theme.hint)`
  - `ColumnarMenu::default().with_text_style(theme.menu_text)...`
- Pass theme's prompt color to `ShannonPrompt`

**`src/prompt.rs`** — use theme colors:

Add `prompt_color: Color` and `indicator_color: Color` and
`error_color: Color` to `ShannonPrompt`. Use them in `get_prompt_color()`
and `get_indicator_color()`.

**`src/lib.rs`** — add `pub mod theme;`

#### Tests

**`src/theme.rs`** tests:

- `test_default_theme` — default theme uses ANSI colors (not RGB)
- `test_parse_color_named` — "green" → Green, "magenta" → Magenta
- `test_parse_color_hex` — "#FF79C6" → Rgb(255, 121, 198)
- `test_parse_color_bright` — "brred" → LightRed
- `test_parse_color_with_modifiers` — "green --bold" → Green bold
- `test_theme_from_config` — overrides apply on top of defaults

#### Verification

1. `cargo build` succeeds.
2. `cargo test` passes.
3. Run shannon with NO `[theme]` config — colors come from terminal's
   ANSI palette. Looks good in any terminal theme.
4. Add overrides to config.toml:
   ```toml
   [theme]
   keyword = "#bb9af7"
   command = "#7aa2f7"
   ```
   Verify those two colors are RGB while others remain ANSI.
5. Syntax highlighting still works for all grammars (bash, nushell, fish).
6. Prompt, hints, and completion menu all respect the theme.
7. Test in a light terminal theme — should be readable (ANSI defaults).

**Result:** Pass

All verification steps confirmed. 84 tests pass (64 unit + 20 integration),
including 8 new theme tests. ANSI defaults work — colors inherit from the
terminal's palette. Individual overrides via config.toml apply correctly
(tested with `keyword = "#bb9af7"`). Syntax highlighting works for all
grammars. Prompt, hints, and highlighter all read from the Theme struct.

#### Conclusion

Theming infrastructure is in place. All hardcoded RGB colors replaced with
ANSI defaults. The Theme struct flows through all components. Config overrides
work. Ready for experiment 2 (named themes from fish `.theme` files).
