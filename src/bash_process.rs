use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use crate::executor;
use crate::shell::ShellState;
use crate::shell_engine::ShellEngine;

const SENTINEL_START: &str = "==SHANNON_SENTINEL_START==";
const SENTINEL_END: &str = "==SHANNON_SENTINEL_END==";

pub struct BashProcess {
    _child: Child,
    stdin: ChildStdin,
    stdout_reader: BufReader<ChildStdout>,
    pending_state: Option<ShellState>,
}

impl BashProcess {
    pub fn new() -> Self {
        let mut child = Command::new("bash")
            .args(["--login"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn bash process");

        let stdin = child.stdin.take().expect("failed to take bash stdin");
        let stdout = child.stdout.take().expect("failed to take bash stdout");
        let stderr = child.stderr.take().expect("failed to take bash stderr");

        // Spawn a thread to forward stderr to real stderr
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        let _ = writeln!(std::io::stderr(), "{line}");
                    }
                    Err(_) => break,
                }
            }
        });

        let mut bp = BashProcess {
            _child: child,
            stdin,
            stdout_reader: BufReader::new(stdout),
            pending_state: None,
        };

        // Trap SIGINT so bash doesn't die when the user presses Ctrl+C.
        // Using `trap 'true' INT` (not `trap '' INT`) ensures children still
        // receive SIGINT with default handling — only SIG_IGN is inherited
        // across exec, not trap actions.
        bp.run_command("trap 'true' INT");

        bp
    }

    /// Capture all exported env vars by running a no-op command.
    pub fn capture_env(&mut self) -> HashMap<String, String> {
        // Run a no-op to trigger the sentinel protocol
        let state = self.run_command("true");
        state.env
    }

    /// Build the env injection preamble (cd + exports) from pending state.
    fn build_preamble(&mut self) -> String {
        let state = match self.pending_state.take() {
            Some(s) => s,
            None => return String::new(),
        };

        let mut preamble = String::new();

        // Change directory
        preamble.push_str(&format!("cd {}\n", shell_escape(&state.cwd.to_string_lossy())));

        // Export env vars
        for (key, value) in &state.env {
            preamble.push_str(&format!("export {}={}\n", key, shell_escape(value)));
        }

        preamble
    }

    /// Send a command with sentinel protocol and read the result.
    fn run_command(&mut self, command: &str) -> ShellState {
        let preamble = self.build_preamble();

        // Build the full command block
        let block = format!(
            "{preamble}{command}\n\
             __shannon_ec=$?\n\
             echo \"{SENTINEL_START}\"\n\
             export -p\n\
             echo \"__SHANNON_CWD=$(pwd)\"\n\
             echo \"__SHANNON_EXIT=$__shannon_ec\"\n\
             echo \"{SENTINEL_END}\"\n"
        );

        // Write to bash's stdin
        if let Err(e) = self.stdin.write_all(block.as_bytes()) {
            eprintln!("shannon: failed to write to bash stdin: {e}");
            return ShellState {
                env: HashMap::new(),
                cwd: std::path::PathBuf::from("/"),
                last_exit_code: 1,
            };
        }
        if let Err(e) = self.stdin.flush() {
            eprintln!("shannon: failed to flush bash stdin: {e}");
            return ShellState {
                env: HashMap::new(),
                cwd: std::path::PathBuf::from("/"),
                last_exit_code: 1,
            };
        }

        // Read stdout line-by-line
        let mut in_sentinel = false;
        let mut sentinel_buf = String::new();
        let mut line = String::new();

        loop {
            line.clear();
            match self.stdout_reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
                    if trimmed == SENTINEL_END {
                        break;
                    } else if trimmed == SENTINEL_START {
                        in_sentinel = true;
                    } else if in_sentinel {
                        sentinel_buf.push_str(trimmed);
                        sentinel_buf.push('\n');
                    } else {
                        // Command output — display to user
                        print!("{}", line);
                        let _ = std::io::stdout().flush();
                    }
                }
                Err(e) => {
                    eprintln!("shannon: error reading bash stdout: {e}");
                    break;
                }
            }
        }

        // Parse sentinel buffer
        let (env, cwd) = executor::parse_bash_env(&sentinel_buf)
            .unwrap_or_else(|| (HashMap::new(), std::path::PathBuf::from("/")));

        // Extract exit code
        let exit_code = sentinel_buf
            .lines()
            .find_map(|l| l.strip_prefix("__SHANNON_EXIT="))
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(1);

        ShellState {
            env,
            cwd,
            last_exit_code: exit_code,
        }
    }
}

impl ShellEngine for BashProcess {
    fn inject_state(&mut self, state: &ShellState) {
        self.pending_state = Some(state.clone());
    }

    fn execute(&mut self, command: &str) -> ShellState {
        self.run_command(command)
    }
}

/// Escape a string for use in a single-quoted bash context.
/// e.g., "it's" becomes "'it'\''s'"
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_escape_simple() {
        assert_eq!(shell_escape("hello"), "'hello'");
    }

    #[test]
    fn test_shell_escape_with_quotes() {
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }

    #[test]
    fn test_shell_escape_with_spaces() {
        assert_eq!(shell_escape("hello world"), "'hello world'");
    }

    #[test]
    fn test_bash_process_echo() {
        let mut bp = BashProcess::new();
        let state = bp.run_command("echo hello");
        assert_eq!(state.last_exit_code, 0);
    }

    #[test]
    fn test_bash_process_env_persistence() {
        let mut bp = BashProcess::new();
        bp.run_command("export TEST_VAR=foobar");
        let state = bp.run_command("echo $TEST_VAR");
        assert_eq!(state.env.get("TEST_VAR").unwrap(), "foobar");
    }

    #[test]
    fn test_bash_process_cwd_persistence() {
        let dir = tempfile::TempDir::new().unwrap();
        let dir_path = dir.path().to_string_lossy().to_string();
        let mut bp = BashProcess::new();
        bp.run_command(&format!("cd {}", shell_escape(&dir_path)));
        let state = bp.run_command("pwd");
        assert_eq!(state.cwd.to_string_lossy(), dir_path);
    }

    #[test]
    fn test_bash_process_exit_code() {
        let mut bp = BashProcess::new();
        let state = bp.run_command("false");
        assert_eq!(state.last_exit_code, 1);
    }

    #[test]
    fn test_bash_process_capture_env() {
        let mut bp = BashProcess::new();
        bp.run_command("export CAPTURE_TEST=works");
        let env = bp.capture_env();
        assert_eq!(env.get("CAPTURE_TEST").unwrap(), "works");
    }

    #[test]
    fn test_bash_process_inject_state() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut env = HashMap::new();
        env.insert("INJECTED".to_string(), "yes".to_string());
        let state = ShellState {
            env,
            cwd: dir.path().to_path_buf(),
            last_exit_code: 0,
        };
        let mut bp = BashProcess::new();
        bp.inject_state(&state);
        let result = bp.run_command("echo $INJECTED");
        assert_eq!(result.env.get("INJECTED").unwrap(), "yes");
        assert_eq!(result.cwd, dir.path());
    }
}
