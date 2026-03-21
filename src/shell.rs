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
            .join("olshell");
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
