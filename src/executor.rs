use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::process::Command;

use crate::shell::{ShellKind, ShellState};

pub fn execute_command(
    shell: ShellKind,
    command: &str,
    state: &ShellState,
) -> io::Result<ShellState> {
    let temp_file = tempfile::Builder::new()
        .prefix("shannon_")
        .suffix(".env")
        .tempfile()?;
    let temp_path = temp_file.path().to_string_lossy().to_string();

    let wrapper = match shell {
        ShellKind::Bash => build_bash_wrapper(command, &temp_path),
        ShellKind::Nushell => build_nushell_wrapper(command, &temp_path),
    };

    let status = Command::new(shell.binary())
        .args(["-c", &wrapper])
        .env_clear()
        .envs(&state.env)
        .current_dir(&state.cwd)
        .status();

    let exit_code = match &status {
        Ok(s) => s.code().unwrap_or(1),
        Err(_) => 1,
    };

    // Try to read captured state; fall back to previous state on failure
    let new_state = std::fs::read_to_string(&temp_path)
        .ok()
        .and_then(|contents| match shell {
            ShellKind::Bash => parse_bash_env(&contents),
            ShellKind::Nushell => parse_nushell_env(&contents),
        })
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

fn build_bash_wrapper(command: &str, temp_path: &str) -> String {
    format!(
        r#"{command}
__shannon_ec=$?
(export -p; echo "__SHANNON_CWD=$(pwd)"; echo "__SHANNON_EXIT=$__shannon_ec") > '{temp_path}'
exit $__shannon_ec"#
    )
}

fn build_nushell_wrapper(command: &str, temp_path: &str) -> String {
    format!(
        r#"let __shannon_out = (try {{ {command} }} catch {{ |e| $e.rendered | print -e; null }})
if ($__shannon_out != null) and (($__shannon_out | describe) != "nothing") {{ $__shannon_out | print }}
let shannon_exit = (if ($env | get -o LAST_EXIT_CODE | is-not-empty) {{ $env.LAST_EXIT_CODE }} else {{ 0 }})
$env | reject config? | insert __SHANNON_CWD (pwd) | insert __SHANNON_EXIT ($shannon_exit | into string) | to json --serialize | save --force '{temp_path}'"#
    )
}

/// Parse bash `export -p` output plus our special __SHANNON_ markers.
fn parse_bash_env(contents: &str) -> Option<(HashMap<String, String>, PathBuf)> {
    let mut env = HashMap::new();
    let mut cwd: Option<PathBuf> = None;

    for line in contents.lines() {
        // Lines from `export -p` look like: declare -x KEY="VALUE"
        // or: declare -x KEY (no value)
        // Our markers look like: __SHANNON_CWD=/some/path
        if let Some(rest) = line.strip_prefix("declare -x ") {
            if let Some((key, value)) = parse_declare_line(rest) {
                if key == "__SHANNON_CWD" {
                    cwd = Some(PathBuf::from(&value));
                } else if key == "__SHANNON_EXIT" {
                    // Skip — we use the process exit code directly
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
        // Strip surrounding quotes if present
        let value =
            if raw_value.starts_with('"') && raw_value.ends_with('"') && raw_value.len() >= 2 {
                unescape_bash_value(&raw_value[1..raw_value.len() - 1])
            } else {
                raw_value.to_string()
            };
        Some((key, value))
    } else {
        // Exported but no value — skip
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
                Some('\n') => {} // line continuation
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
            // Nushell stores PATH (and similar) as a list — join with path separator
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
        // Other non-string values are silently dropped (strings-only policy)
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

    // --- build wrappers ---

    #[test]
    fn test_build_bash_wrapper() {
        let wrapper = build_bash_wrapper("echo hello", "/tmp/state.env");
        assert!(wrapper.contains("echo hello"));
        assert!(wrapper.contains("/tmp/state.env"));
        assert!(wrapper.contains("export -p"));
        assert!(wrapper.contains("__SHANNON_CWD"));
    }

    #[test]
    fn test_build_nushell_wrapper() {
        let wrapper = build_nushell_wrapper("echo hello", "/tmp/state.env");
        assert!(wrapper.contains("echo hello"));
        assert!(wrapper.contains("/tmp/state.env"));
        assert!(wrapper.contains("__SHANNON_CWD"));
        assert!(wrapper.contains("to json"));
    }
}
