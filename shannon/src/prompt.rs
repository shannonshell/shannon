use std::borrow::Cow;
use std::path::PathBuf;

use crossterm::style::Color;
use reedline::{Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus};

pub struct ShannonPrompt {
    pub shell_name: String,
    pub cwd: PathBuf,
    pub last_exit_code: i32,
    pub depth: u32,
    pub ai_mode: bool,
    pub prompt_color: Color,
    pub indicator_color: Color,
    pub error_color: Color,
    pub ai_badge_style: nu_ansi_term::Style,
}

/// Tilde-contract a path (replace home dir prefix with ~).
pub fn tilde_contract(cwd: &PathBuf) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(rest) = cwd.strip_prefix(&home) {
            if rest.as_os_str().is_empty() {
                return "~".to_string();
            }
            return format!("~/{}", rest.display());
        }
    }
    cwd.display().to_string()
}

impl ShannonPrompt {
    fn tilde_contract(&self) -> String {
        tilde_contract(&self.cwd)
    }
}

impl Prompt for ShannonPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        if self.ai_mode {
            let badge = self.ai_badge_style.paint(" AI ").to_string();
            // Re-apply prompt color after badge's ANSI reset
            let prompt_ansi = match self.prompt_color {
                Color::Rgb { r, g, b } => format!("\x1b[38;2;{r};{g};{b}m"),
                Color::Cyan => "\x1b[36m".to_string(),
                Color::Green => "\x1b[32m".to_string(),
                Color::Yellow => "\x1b[33m".to_string(),
                Color::Blue => "\x1b[34m".to_string(),
                Color::Magenta => "\x1b[35m".to_string(),
                Color::Red => "\x1b[31m".to_string(),
                _ => "\x1b[36m".to_string(), // default cyan
            };
            Cow::Owned(format!(
                "{} {}[{}] {}",
                badge,
                prompt_ansi,
                self.shell_name,
                self.tilde_contract()
            ))
        } else {
            Cow::Owned(format!(
                "[{}] {}",
                self.shell_name,
                self.tilde_contract()
            ))
        }
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _prompt_mode: PromptEditMode) -> Cow<'_, str> {
        let depth_prefix = if self.depth > 1 {
            ">".repeat((self.depth - 1) as usize)
        } else {
            String::new()
        };
        if self.last_exit_code != 0 && self.last_exit_code < 128 {
            Cow::Owned(format!(" {depth_prefix}! "))
        } else {
            Cow::Owned(format!(" {depth_prefix}> "))
        }
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed(":: ")
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        let prefix = match history_search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };
        Cow::Owned(format!(
            "({}reverse-search: {}) ",
            prefix, history_search.term
        ))
    }

    fn get_prompt_color(&self) -> Color {
        self.prompt_color
    }

    fn get_indicator_color(&self) -> Color {
        if self.last_exit_code != 0 && self.last_exit_code < 128 {
            self.error_color
        } else {
            self.indicator_color
        }
    }
}
