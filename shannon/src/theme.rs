use std::collections::HashMap;
use std::sync::LazyLock;

use nu_ansi_term::{Color, Style};

use crate::config::ThemeConfig;

/// Embedded themes parsed at build time from themes/*.theme files.
/// Map: theme_name → { section_name → { category → color_string } }
static THEMES: LazyLock<HashMap<String, HashMap<String, HashMap<String, String>>>> =
    LazyLock::new(|| {
        let json = include_str!(concat!(env!("OUT_DIR"), "/themes.json"));
        serde_json::from_str(json).unwrap_or_default()
    });

/// Shannon's theme — semantic colors for syntax highlighting and UI.
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

    // UI (crossterm colors for reedline Prompt trait)
    pub prompt: crossterm::style::Color,
    pub prompt_error: crossterm::style::Color,
    pub prompt_indicator: crossterm::style::Color,

    // UI (nu_ansi_term styles for hinter/menu)
    pub hint: Style,
    pub ai_badge: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            keyword: Style::new().fg(Color::Magenta).bold(),
            command: Style::new().fg(Color::Blue),
            string: Style::new().fg(Color::Green),
            number: Style::new().fg(Color::Yellow),
            variable: Style::new().fg(Color::Cyan),
            operator: Style::new().fg(Color::Cyan),
            comment: Style::new().fg(Color::DarkGray),
            error: Style::new().fg(Color::Red).bold(),
            foreground: Style::new().fg(Color::White),
            type_: Style::new().fg(Color::Yellow),
            prompt: crossterm::style::Color::Cyan,
            prompt_error: crossterm::style::Color::Red,
            prompt_indicator: crossterm::style::Color::DarkGrey,
            hint: Style::new().fg(Color::DarkGray).italic(),
            ai_badge: Style::new().fg(Color::Black).on(Color::Magenta),
        }
    }
}

impl Theme {
    /// Apply a named theme's colors on top of the current theme.
    fn apply_named_theme(&mut self, name: &str) {
        let themes = &*THEMES;
        let sections = match themes.get(name) {
            Some(s) => s,
            None => {
                eprintln!("shannon: unknown theme: {name}");
                return;
            }
        };

        // Prefer "dark" section, fall back to "unknown", then first available
        let colors = sections
            .get("dark")
            .or_else(|| sections.get("unknown"))
            .or_else(|| sections.values().next());

        let colors = match colors {
            Some(c) => c,
            None => return,
        };

        for (category, color_str) in colors {
            match category.as_str() {
                "keyword" => self.keyword = parse_style(color_str),
                "command" => self.command = parse_style(color_str),
                "string" => self.string = parse_style(color_str),
                "number" => self.number = parse_style(color_str),
                "variable" => self.variable = parse_style(color_str),
                "operator" => self.operator = parse_style(color_str),
                "comment" => self.comment = parse_style(color_str),
                "error" => self.error = parse_style(color_str),
                "foreground" => self.foreground = parse_style(color_str),
                "hint" => self.hint = parse_style(color_str),
                "prompt" => self.prompt = parse_crossterm_color(color_str),
                _ => {} // menu colors etc. — future use
            }
        }
    }

    /// Create a theme from config, applying overrides on top of defaults.
    pub fn from_config(config: &ThemeConfig) -> Self {
        let mut theme = Theme::default();

        // Layer 2: named theme
        if let Some(ref name) = config.name {
            theme.apply_named_theme(name);
        }

        // Layer 3: individual overrides
        if let Some(ref s) = config.keyword {
            theme.keyword = parse_style(s);
        }
        if let Some(ref s) = config.command {
            theme.command = parse_style(s);
        }
        if let Some(ref s) = config.string {
            theme.string = parse_style(s);
        }
        if let Some(ref s) = config.number {
            theme.number = parse_style(s);
        }
        if let Some(ref s) = config.variable {
            theme.variable = parse_style(s);
        }
        if let Some(ref s) = config.operator {
            theme.operator = parse_style(s);
        }
        if let Some(ref s) = config.comment {
            theme.comment = parse_style(s);
        }
        if let Some(ref s) = config.error {
            theme.error = parse_style(s);
        }
        if let Some(ref s) = config.foreground {
            theme.foreground = parse_style(s);
        }
        if let Some(ref s) = config.type_ {
            theme.type_ = parse_style(s);
        }
        if let Some(ref s) = config.prompt {
            theme.prompt = parse_crossterm_color(s);
        }
        if let Some(ref s) = config.hint {
            theme.hint = parse_style(s);
        }
        if let Some(ref s) = config.ai_badge {
            theme.ai_badge = parse_style(s);
        }

        theme
    }
}

/// Parse a color string into a nu_ansi_term Style.
/// Supports: named ("green"), bright ("brgreen"), hex ("#FF79C6"),
/// and modifiers ("green --bold", "cyan --italic").
pub fn parse_style(s: &str) -> Style {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.is_empty() {
        return Style::default();
    }

    let color_str = parts[0];
    let mut style = if color_str.starts_with('-') {
        // No color, just modifiers
        Style::default()
    } else {
        Style::new().fg(parse_nu_color(color_str))
    };

    // Apply modifiers
    for part in &parts[1..] {
        match *part {
            "--bold" => style = style.bold(),
            "--italic" | "--italics" => style = style.italic(),
            "--underline" => style = style.underline(),
            "--dimmed" => style = style.dimmed(),
            "--reverse" => style = style.reverse(),
            "--strikethrough" => style = style.strikethrough(),
            _ => {}
        }
    }

    style
}

/// Parse a color name or hex value into a nu_ansi_term Color.
fn parse_nu_color(s: &str) -> Color {
    // Hex color
    let hex = s.strip_prefix('#').unwrap_or(s);
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        return Color::Rgb(r, g, b);
    }

    // Named colors
    match s.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" | "purple" => Color::Purple,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "darkgray" | "dark_gray" => Color::DarkGray,
        "brred" | "light_red" => Color::LightRed,
        "brgreen" | "light_green" => Color::LightGreen,
        "bryellow" | "light_yellow" => Color::LightYellow,
        "brblue" | "light_blue" => Color::LightBlue,
        "brmagenta" | "brpurple" | "light_purple" => Color::LightPurple,
        "brcyan" | "light_cyan" => Color::LightCyan,
        "brwhite" | "light_gray" => Color::LightGray,
        "default" | "normal" => Color::White,
        _ => Color::White,
    }
}

/// Parse a color string into a crossterm Color (for reedline Prompt trait).
fn parse_crossterm_color(s: &str) -> crossterm::style::Color {
    let color_str = s.split_whitespace().next().unwrap_or(s);

    // Hex
    let hex = color_str.strip_prefix('#').unwrap_or(color_str);
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        return crossterm::style::Color::Rgb { r, g, b };
    }

    match color_str.to_lowercase().as_str() {
        "black" => crossterm::style::Color::Black,
        "red" => crossterm::style::Color::Red,
        "green" => crossterm::style::Color::Green,
        "yellow" => crossterm::style::Color::Yellow,
        "blue" => crossterm::style::Color::Blue,
        "magenta" | "purple" => crossterm::style::Color::Magenta,
        "cyan" => crossterm::style::Color::Cyan,
        "white" => crossterm::style::Color::White,
        "darkgray" | "dark_gray" => crossterm::style::Color::DarkGrey,
        "brred" | "light_red" => crossterm::style::Color::Red,
        "brgreen" | "light_green" => crossterm::style::Color::Green,
        _ => crossterm::style::Color::Cyan,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_theme_uses_ansi() {
        let theme = Theme::default();
        // Verify colors are ANSI, not RGB
        assert_eq!(theme.keyword, Style::new().fg(Color::Magenta).bold());
        assert_eq!(theme.command, Style::new().fg(Color::Blue));
        assert_eq!(theme.string, Style::new().fg(Color::Green));
    }

    #[test]
    fn test_parse_style_named() {
        let style = parse_style("green");
        assert_eq!(style, Style::new().fg(Color::Green));
    }

    #[test]
    fn test_parse_style_hex() {
        let style = parse_style("#FF79C6");
        assert_eq!(style, Style::new().fg(Color::Rgb(255, 121, 198)));
    }

    #[test]
    fn test_parse_style_hex_no_hash() {
        let style = parse_style("FF79C6");
        assert_eq!(style, Style::new().fg(Color::Rgb(255, 121, 198)));
    }

    #[test]
    fn test_parse_style_bright() {
        let style = parse_style("brred");
        assert_eq!(style, Style::new().fg(Color::LightRed));
    }

    #[test]
    fn test_parse_style_with_modifiers() {
        let style = parse_style("green --bold");
        assert_eq!(style, Style::new().fg(Color::Green).bold());
    }

    #[test]
    fn test_parse_style_multiple_modifiers() {
        let style = parse_style("cyan --bold --italic");
        assert_eq!(style, Style::new().fg(Color::Cyan).bold().italic());
    }

    #[test]
    fn test_theme_from_config_overrides() {
        let config = ThemeConfig {
            keyword: Some("#bb9af7".to_string()),
            command: Some("blue --bold".to_string()),
            ..Default::default()
        };
        let theme = Theme::from_config(&config);
        assert_eq!(theme.keyword, Style::new().fg(Color::Rgb(187, 154, 247)));
        assert_eq!(theme.command, Style::new().fg(Color::Blue).bold());
        // Non-overridden fields stay as ANSI defaults
        assert_eq!(theme.string, Style::new().fg(Color::Green));
    }

    #[test]
    fn test_themes_loaded() {
        let themes = &*THEMES;
        assert!(
            themes.len() > 20,
            "expected 20+ themes, got {}",
            themes.len()
        );
        assert!(themes.contains_key("tokyo-night"));
        assert!(themes.contains_key("catppuccin-mocha"));
        assert!(themes.contains_key("dracula"));
    }

    #[test]
    fn test_named_theme_tokyo_night() {
        let config = ThemeConfig {
            name: Some("tokyo-night".to_string()),
            ..Default::default()
        };
        let theme = Theme::from_config(&config);
        // Tokyo Night keyword is #bb9af7
        assert_eq!(theme.keyword, Style::new().fg(Color::Rgb(187, 154, 247)));
        // Tokyo Night command is #7aa2f7
        assert_eq!(theme.command, Style::new().fg(Color::Rgb(122, 162, 247)));
    }

    #[test]
    fn test_named_theme_with_override() {
        let config = ThemeConfig {
            name: Some("tokyo-night".to_string()),
            keyword: Some("red".to_string()),
            ..Default::default()
        };
        let theme = Theme::from_config(&config);
        // Override takes precedence
        assert_eq!(theme.keyword, Style::new().fg(Color::Red));
        // Non-overridden stays from named theme
        assert_eq!(theme.command, Style::new().fg(Color::Rgb(122, 162, 247)));
    }

    #[test]
    fn test_no_config_stays_ansi() {
        let config = ThemeConfig::default();
        let theme = Theme::from_config(&config);
        assert_eq!(theme.keyword, Style::new().fg(Color::Magenta).bold());
        assert_eq!(theme.command, Style::new().fg(Color::Blue));
    }
}
