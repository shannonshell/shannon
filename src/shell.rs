use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Bash,
    Nushell,
}

impl ShellKind {
    pub fn display_name(&self) -> &str {
        match self {
            ShellKind::Bash => "bash",
            ShellKind::Nushell => "nu",
        }
    }

    pub fn binary(&self) -> &str {
        match self {
            ShellKind::Bash => "bash",
            ShellKind::Nushell => "nu",
        }
    }

    pub fn history_file(&self) -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("shannon");
        match self {
            ShellKind::Bash => config_dir.join("bash_history"),
            ShellKind::Nushell => config_dir.join("nu_history"),
        }
    }
}

pub struct ShellState {
    pub env: HashMap<String, String>,
    pub cwd: PathBuf,
    pub last_exit_code: i32,
}

impl ShellState {
    pub fn from_current_env() -> Self {
        ShellState {
            env: std::env::vars().collect(),
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            last_exit_code: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_kind_display_name() {
        assert_eq!(ShellKind::Bash.display_name(), "bash");
        assert_eq!(ShellKind::Nushell.display_name(), "nu");
    }

    #[test]
    fn test_shell_kind_binary() {
        assert_eq!(ShellKind::Bash.binary(), "bash");
        assert_eq!(ShellKind::Nushell.binary(), "nu");
    }

    #[test]
    fn test_shell_kind_history_file() {
        let bash_path = ShellKind::Bash.history_file();
        let nu_path = ShellKind::Nushell.history_file();
        assert!(bash_path.ends_with("shannon/bash_history"));
        assert!(nu_path.ends_with("shannon/nu_history"));
    }

    #[test]
    fn test_shell_state_from_current_env() {
        let state = ShellState::from_current_env();
        assert!(state.env.contains_key("PATH"));
        assert!(state.cwd.is_absolute());
        assert_eq!(state.last_exit_code, 0);
    }
}
