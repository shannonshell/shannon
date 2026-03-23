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
