use std::collections::HashMap;
use std::path::PathBuf;

use nu_cli::{ModeDispatcher, ModeResult};

use crate::bash_process::BashProcess;
use crate::shell::ShellState;
use crate::shell_engine::ShellEngine;

pub struct ShannonDispatcher {
    bash: BashProcess,
}

impl ShannonDispatcher {
    pub fn new() -> Self {
        let bash = BashProcess::new();
        ShannonDispatcher { bash }
    }

    /// Get the current env vars from bash (after login initialization).
    /// Used to inject bash env vars into nushell's Stack at startup.
    pub fn env_vars(&mut self) -> HashMap<String, String> {
        self.bash.capture_env()
    }
}

impl ModeDispatcher for ShannonDispatcher {
    fn execute(
        &mut self,
        mode: &str,
        command: &str,
        env: HashMap<String, String>,
        cwd: PathBuf,
    ) -> ModeResult {
        let state = ShellState {
            env,
            cwd,
            last_exit_code: 0,
        };
        match mode {
            "bash" => {
                self.bash.inject_state(&state);
                let result = self.bash.execute(command);
                ModeResult {
                    env: result.env,
                    cwd: result.cwd,
                    exit_code: result.last_exit_code,
                }
            }
            _ => ModeResult {
                env: state.env,
                cwd: state.cwd,
                exit_code: 127,
            },
        }
    }
}
