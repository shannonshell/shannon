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
        let mut bash = BashProcess::new();

        // Source env.sh in the persistent bash process so bash functions
        // (like nvm) persist across commands in bash mode.
        let config_dir = crate::shell::config_dir();
        let env_sh = config_dir.join("env.sh");
        if env_sh.exists() {
            let state = ShellState::from_current_env();
            bash.inject_state(&state);
            let source_cmd = format!(". '{}'", env_sh.display());
            bash.execute(&source_cmd);
        }

        ShannonDispatcher { bash }
    }

    /// Get the current env vars from bash (after sourcing env.sh).
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
