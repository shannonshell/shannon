use std::borrow::Cow;
use std::path::PathBuf;

use crossterm::style::Color;
use reedline::{Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus};

use crate::shell::ShellKind;

pub struct OlshellPrompt {
    pub shell: ShellKind,
    pub cwd: PathBuf,
    pub last_exit_code: i32,
}

impl OlshellPrompt {
    fn tilde_contract(&self) -> String {
        if let Some(home) = dirs::home_dir() {
            if let Ok(rest) = self.cwd.strip_prefix(&home) {
                if rest.as_os_str().is_empty() {
                    return "~".to_string();
                }
                return format!("~/{}", rest.display());
            }
        }
        self.cwd.display().to_string()
    }
}

impl Prompt for OlshellPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Owned(format!("[{}] {}", self.shell.display_name(), self.tilde_contract()))
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _prompt_mode: PromptEditMode) -> Cow<'_, str> {
        if self.last_exit_code != 0 {
            Cow::Borrowed(" ! ")
        } else {
            Cow::Borrowed(" > ")
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
        Cow::Owned(format!("({}reverse-search: {}) ", prefix, history_search.term))
    }

    fn get_prompt_color(&self) -> Color {
        match self.shell {
            ShellKind::Bash => Color::Green,
            ShellKind::Nushell => Color::Cyan,
        }
    }

    fn get_indicator_color(&self) -> Color {
        if self.last_exit_code != 0 {
            Color::Red
        } else {
            Color::DarkGrey
        }
    }
}
