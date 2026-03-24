use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::process::Command;

use crate::config::{expand_wrapper, read_init_file, ShellConfig};
use crate::shell::ShellState;

pub fn execute_command(
    shell_config: &ShellConfig,
    command: &str,
    state: &ShellState,
) -> io::Result<ShellState> {
    let temp_file = tempfile::Builder::new()
        .prefix("shannon_")
        .suffix(".env")
        .tempfile()?;
    let temp_path = temp_file.path().to_string_lossy().to_string();

    let init_content = read_init_file(shell_config.init.as_deref());
    let wrapper = expand_wrapper(&shell_config.wrapper, command, &temp_path, &init_content);

    // Ignore SIGINT while child runs — let only the child handle it.
    // This is what bash/zsh/fish do: the shell survives Ctrl+C.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_IGN);
    }

    let status = Command::new(&shell_config.binary)
        .args(["-c", &wrapper])
        .env_clear()
        .envs(&state.env)
        .current_dir(&state.cwd)
        .status();

    // Restore default SIGINT handling
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_DFL);
    }

    let exit_code = match &status {
        Ok(s) => s.code().unwrap_or(1),
        Err(_) => 1,
    };

    // Try to read captured state; fall back to previous state on failure
    let new_state = std::fs::read_to_string(&temp_path)
        .ok()
        .and_then(|contents| parse_output(&shell_config.parser, &contents))
        .map(|(env, cwd)| ShellState {
            env,
            cwd,
            last_exit_code: exit_code,
        })
        .unwrap_or_else(|| ShellState {
            env: state.env.clone(),
            cwd: state.cwd.clone(),
            last_exit_code: exit_code,
        });

    // Propagate spawn errors after we've built state
    status?;

    Ok(new_state)
}

/// Dispatch to the correct parser based on the parser name.
fn parse_output(parser: &str, contents: &str) -> Option<(HashMap<String, String>, PathBuf)> {
    match parser {
        "bash" => parse_bash_env(contents),
        "nushell" => parse_nushell_env(contents),
        _ => parse_env(contents),
    }
}

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

    // Use the bash wrapper template directly for the startup script
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

/// Parse `env` output (KEY=VALUE per line) plus __SHANNON_ markers.
/// This is the generic parser used by fish, zsh, and any POSIX shell.
pub fn parse_env(contents: &str) -> Option<(HashMap<String, String>, PathBuf)> {
    let mut env = HashMap::new();
    let mut cwd: Option<PathBuf> = None;

    for line in contents.lines() {
        if let Some(eq_pos) = line.find('=') {
            let key = &line[..eq_pos];
            let value = &line[eq_pos + 1..];
            if key == "__SHANNON_CWD" {
                cwd = Some(PathBuf::from(value));
            } else if key == "__SHANNON_EXIT" {
                // Skip — we use the process exit code directly
            } else if !key.starts_with("__SHANNON_") {
                env.insert(key.to_string(), value.to_string());
            }
        }
    }

    Some((env, cwd.unwrap_or_else(|| PathBuf::from("/"))))
}

/// Parse bash `export -p` output plus our special __SHANNON_ markers.
fn parse_bash_env(contents: &str) -> Option<(HashMap<String, String>, PathBuf)> {
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

/// Parse a single `KEY="VALUE"` or `KEY` from a declare -x line.
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

/// Unescape common bash escape sequences in double-quoted strings.
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

/// Parse nushell JSON env output.
fn parse_nushell_env(contents: &str) -> Option<(HashMap<String, String>, PathBuf)> {
    let obj: serde_json::Value = serde_json::from_str(contents).ok()?;
    let map = obj.as_object()?;

    let mut env = HashMap::new();
    let mut cwd: Option<PathBuf> = None;

    for (key, value) in map {
        if key == "__SHANNON_CWD" {
            if let Some(s) = value.as_str() {
                cwd = Some(PathBuf::from(s));
            }
        } else if key == "__SHANNON_EXIT" {
            // Skip
        } else if let Some(s) = value.as_str() {
            env.insert(key.clone(), s.to_string());
        } else if let Some(arr) = value.as_array() {
            let all_strings = arr.iter().all(|v| v.is_string());
            if all_strings {
                let joined = arr
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(if cfg!(windows) { ";" } else { ":" });
                env.insert(key.clone(), joined);
            }
        }
    }

    Some((env, cwd.unwrap_or_else(|| PathBuf::from("/"))))
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_bash_env ---

    #[test]
    fn test_parse_bash_env_basic() {
        let input = r#"declare -x HOME="/Users/ryan"
declare -x PATH="/usr/bin:/bin"
declare -x TERM="xterm-256color"
__SHANNON_CWD=/tmp
__SHANNON_EXIT=0"#;
        let (env, cwd) = parse_bash_env(input).unwrap();
        assert_eq!(env.get("HOME").unwrap(), "/Users/ryan");
        assert_eq!(env.get("PATH").unwrap(), "/usr/bin:/bin");
        assert_eq!(env.get("TERM").unwrap(), "xterm-256color");
        assert_eq!(cwd, PathBuf::from("/tmp"));
        assert!(!env.contains_key("__SHANNON_CWD"));
        assert!(!env.contains_key("__SHANNON_EXIT"));
    }

    #[test]
    fn test_parse_bash_env_shannon_markers_in_declare() {
        let input = r#"declare -x FOO="bar"
declare -x __SHANNON_CWD="/home/user"
declare -x __SHANNON_EXIT="0""#;
        let (env, cwd) = parse_bash_env(input).unwrap();
        assert_eq!(env.get("FOO").unwrap(), "bar");
        assert_eq!(cwd, PathBuf::from("/home/user"));
        assert!(!env.contains_key("__SHANNON_CWD"));
        assert!(!env.contains_key("__SHANNON_EXIT"));
    }

    #[test]
    fn test_parse_bash_env_quoted_values() {
        let input = r#"declare -x MSG="hello \"world\""
declare -x DOLLAR="price is \$5"
declare -x BACK="a\\b"
__SHANNON_CWD=/"#;
        let (env, _) = parse_bash_env(input).unwrap();
        assert_eq!(env.get("MSG").unwrap(), r#"hello "world""#);
        assert_eq!(env.get("DOLLAR").unwrap(), "price is $5");
        assert_eq!(env.get("BACK").unwrap(), "a\\b");
    }

    #[test]
    fn test_parse_bash_env_empty() {
        let (env, cwd) = parse_bash_env("").unwrap();
        assert!(env.is_empty());
        assert_eq!(cwd, PathBuf::from("/"));
    }

    #[test]
    fn test_parse_bash_env_no_value() {
        let input = "declare -x EXPORTED_BUT_UNSET\n__SHANNON_CWD=/tmp";
        let (env, _) = parse_bash_env(input).unwrap();
        assert!(!env.contains_key("EXPORTED_BUT_UNSET"));
    }

    // --- unescape_bash_value ---

    #[test]
    fn test_unescape_bash_value() {
        assert_eq!(
            unescape_bash_value(r#"hello \"world\""#),
            r#"hello "world""#
        );
        assert_eq!(unescape_bash_value(r"a\\b"), "a\\b");
        assert_eq!(unescape_bash_value(r"\$HOME"), "$HOME");
        assert_eq!(unescape_bash_value(r"back\`tick"), "back`tick");
        assert_eq!(unescape_bash_value("no escapes"), "no escapes");
        assert_eq!(unescape_bash_value(r"trailing\"), "trailing\\");
    }

    // --- parse_nushell_env ---

    #[test]
    fn test_parse_nushell_env_basic() {
        let input = r#"{"HOME": "/Users/ryan", "TERM": "xterm", "__SHANNON_CWD": "/tmp", "__SHANNON_EXIT": "0"}"#;
        let (env, cwd) = parse_nushell_env(input).unwrap();
        assert_eq!(env.get("HOME").unwrap(), "/Users/ryan");
        assert_eq!(env.get("TERM").unwrap(), "xterm");
        assert_eq!(cwd, PathBuf::from("/tmp"));
        assert!(!env.contains_key("__SHANNON_CWD"));
        assert!(!env.contains_key("__SHANNON_EXIT"));
    }

    #[test]
    fn test_parse_nushell_env_arrays() {
        let input = r#"{"PATH": ["/usr/bin", "/bin", "/usr/local/bin"], "__SHANNON_CWD": "/home"}"#;
        let (env, _) = parse_nushell_env(input).unwrap();
        assert_eq!(env.get("PATH").unwrap(), "/usr/bin:/bin:/usr/local/bin");
    }

    #[test]
    fn test_parse_nushell_env_non_string_dropped() {
        let input =
            r#"{"FOO": "bar", "NUM": 42, "OBJ": {"a": 1}, "BOOL": true, "__SHANNON_CWD": "/"}"#;
        let (env, _) = parse_nushell_env(input).unwrap();
        assert_eq!(env.get("FOO").unwrap(), "bar");
        assert!(!env.contains_key("NUM"));
        assert!(!env.contains_key("OBJ"));
        assert!(!env.contains_key("BOOL"));
    }

    #[test]
    fn test_parse_nushell_env_invalid_json() {
        assert!(parse_nushell_env("not json at all").is_none());
        assert!(parse_nushell_env("").is_none());
    }

    // --- parse_env (generic KEY=VALUE) ---

    #[test]
    fn test_parse_env_basic() {
        let input = "HOME=/Users/ryan\nPATH=/usr/bin:/bin\nTERM=xterm\n__SHANNON_CWD=/tmp\n__SHANNON_EXIT=0";
        let (env, cwd) = parse_env(input).unwrap();
        assert_eq!(env.get("HOME").unwrap(), "/Users/ryan");
        assert_eq!(env.get("PATH").unwrap(), "/usr/bin:/bin");
        assert_eq!(env.get("TERM").unwrap(), "xterm");
        assert_eq!(cwd, PathBuf::from("/tmp"));
        assert!(!env.contains_key("__SHANNON_CWD"));
        assert!(!env.contains_key("__SHANNON_EXIT"));
    }

    #[test]
    fn test_parse_env_empty() {
        let (env, cwd) = parse_env("").unwrap();
        assert!(env.is_empty());
        assert_eq!(cwd, PathBuf::from("/"));
    }

    // --- run_startup_script ---

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
        assert!(result.env.contains_key("HOME"), "HOME should still exist");
        assert!(result.env.contains_key("PATH"), "PATH should still exist");
    }

    #[test]
    fn test_run_startup_script_path_append() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = dir.path().join("config.sh");
        std::fs::write(&config, "export PATH=\"$PATH:/custom/bin\"\n").unwrap();
        let state = make_startup_state(dir.path());
        let result = run_startup_script_from(state, Some(config));
        let path = result.env.get("PATH").unwrap();
        assert!(path.contains("/custom/bin"), "PATH should contain /custom/bin, got: {path}");
        assert!(path.contains("/usr/bin"), "PATH should still contain /usr/bin, got: {path}");
    }
}
