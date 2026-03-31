use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use crate::shell::ShellState;

/// Run the optional startup script at env.sh (or config.sh fallback).
/// Captures the resulting environment and returns an updated ShellState.
/// If the file doesn't exist or fails, returns the original state.
pub fn run_startup_script(state: ShellState) -> ShellState {
    run_startup_script_from(state, None)
}

/// Inner implementation that accepts an optional config path override (for testing).
fn run_startup_script_from(state: ShellState, config_path: Option<PathBuf>) -> ShellState {
    let config_file = config_path.unwrap_or_else(|| {
        let dir = crate::shell::config_dir();
        let env_sh = dir.join("env.sh");
        if env_sh.exists() {
            env_sh
        } else {
            dir.join("config.sh") // backward compatibility
        }
    });

    if !config_file.exists() {
        return state;
    }

    let config_str = config_file.to_string_lossy();

    let temp_file = match tempfile::Builder::new()
        .prefix("shannon_startup_")
        .suffix(".env")
        .tempfile()
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("shannon: failed to create temp file for env script: {e}");
            return state;
        }
    };
    let temp_path = temp_file.path().to_string_lossy().to_string();

    let wrapper = format!(
        "source '{config_str}'\n__shannon_ec=$?\n(export -p; echo \"__SHANNON_CWD=$(pwd)\"; echo \"__SHANNON_EXIT=$__shannon_ec\") > '{temp_path}'\nexit $__shannon_ec"
    );

    let status = Command::new("bash")
        .args(["-c", &wrapper])
        .env_clear()
        .envs(&state.env)
        .current_dir(&state.cwd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status();

    match status {
        Ok(s) if !s.success() => {
            eprintln!(
                "shannon: env script exited with code {} (continuing with inherited env)",
                s.code().unwrap_or(-1)
            );
            return state;
        }
        Err(e) => {
            eprintln!("shannon: failed to run env script: {e}");
            return state;
        }
        _ => {}
    }

    match std::fs::read_to_string(&temp_path)
        .ok()
        .and_then(|contents| parse_bash_env(&contents))
    {
        Some((env, _cwd)) => ShellState {
            env,
            cwd: state.cwd,
            last_exit_code: 0,
        },
        None => {
            eprintln!("shannon: failed to parse env script output (continuing with inherited env)");
            state
        }
    }
}

/// Parse bash `export -p` output plus __SHANNON_ markers.
pub fn parse_bash_env(contents: &str) -> Option<(HashMap<String, String>, PathBuf)> {
    let mut env = HashMap::new();
    let mut cwd: Option<PathBuf> = None;

    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("declare -x ") {
            if let Some((key, value)) = parse_declare_line(rest) {
                if key == "__SHANNON_CWD" {
                    cwd = Some(PathBuf::from(&value));
                } else if key == "__SHANNON_EXIT" {
                    // Skip
                } else {
                    env.insert(key, value);
                }
            }
        } else if let Some(rest) = line.strip_prefix("__SHANNON_CWD=") {
            cwd = Some(PathBuf::from(rest));
        } else if line.starts_with("__SHANNON_EXIT=") {
            // Skip
        }
    }

    Some((env, cwd.unwrap_or_else(|| PathBuf::from("/"))))
}

fn parse_declare_line(s: &str) -> Option<(String, String)> {
    if let Some(eq_pos) = s.find('=') {
        let key = s[..eq_pos].to_string();
        let raw_value = &s[eq_pos + 1..];
        let value =
            if raw_value.starts_with('"') && raw_value.ends_with('"') && raw_value.len() >= 2 {
                unescape_bash_value(&raw_value[1..raw_value.len() - 1])
            } else {
                raw_value.to_string()
            };
        Some((key, value))
    } else {
        None
    }
}

fn unescape_bash_value(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some('$') => result.push('$'),
                Some('`') => result.push('`'),
                Some('\n') => {}
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_startup_state(dir: &std::path::Path) -> ShellState {
        let mut env = HashMap::new();
        env.insert("HOME".to_string(), dir.to_string_lossy().to_string());
        env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
        ShellState {
            env,
            cwd: dir.to_path_buf(),
            last_exit_code: 0,
        }
    }

    #[test]
    fn test_run_startup_script_with_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = dir.path().join("config.sh");
        std::fs::write(&config, "export SHANNON_TEST=from_config\n").unwrap();
        let state = make_startup_state(dir.path());
        let result = run_startup_script_from(state, Some(config));
        assert_eq!(result.env.get("SHANNON_TEST").unwrap(), "from_config");
    }

    #[test]
    fn test_run_startup_script_missing_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = dir.path().join("config.sh");
        let state = make_startup_state(dir.path());
        let original_env = state.env.clone();
        let result = run_startup_script_from(state, Some(config));
        assert_eq!(result.env, original_env);
    }

    #[test]
    fn test_run_startup_script_bad_script() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = dir.path().join("config.sh");
        std::fs::write(&config, "exit 1\n").unwrap();
        let state = make_startup_state(dir.path());
        let original_env = state.env.clone();
        let result = run_startup_script_from(state, Some(config));
        assert_eq!(result.env, original_env);
    }

    #[test]
    fn test_run_startup_script_preserves_existing_env() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = dir.path().join("config.sh");
        std::fs::write(&config, "export NEW_VAR=hello\n").unwrap();
        let state = make_startup_state(dir.path());
        let result = run_startup_script_from(state, Some(config));
        assert_eq!(result.env.get("NEW_VAR").unwrap(), "hello");
        assert!(result.env.contains_key("HOME"));
        assert!(result.env.contains_key("PATH"));
    }

    #[test]
    fn test_run_startup_script_path_append() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = dir.path().join("config.sh");
        std::fs::write(&config, "export PATH=\"$PATH:/custom/bin\"\n").unwrap();
        let state = make_startup_state(dir.path());
        let result = run_startup_script_from(state, Some(config));
        let path = result.env.get("PATH").unwrap();
        assert!(path.contains("/custom/bin"));
        assert!(path.contains("/usr/bin"));
    }
}
