use std::collections::HashMap;
use std::path::PathBuf;

/// Returns the shannon config directory, respecting XDG_CONFIG_HOME.
/// Falls back to ~/.config/shannon.
pub fn config_dir() -> PathBuf {
    let base = match std::env::var("XDG_CONFIG_HOME") {
        Ok(val) if !val.is_empty() => PathBuf::from(val),
        _ => dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config"),
    };
    base.join("shannon")
}

/// Returns the path to the shared SQLite history database.
pub fn history_db() -> PathBuf {
    config_dir().join("history.db")
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
    fn test_shell_state_from_current_env() {
        let state = ShellState::from_current_env();
        assert!(state.env.contains_key("PATH"));
        assert!(state.cwd.is_absolute());
        assert_eq!(state.last_exit_code, 0);
    }
}
