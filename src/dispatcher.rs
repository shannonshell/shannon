use std::collections::HashMap;
use std::path::PathBuf;

use nu_cli::{ModeDispatcher, ModeResult};

use crate::brush_engine::BrushEngine;
use crate::shell::ShellState;
use crate::shell_engine::ShellEngine;

pub struct ShannonDispatcher {
    brush: BrushEngine,
}

impl ShannonDispatcher {
    pub fn new() -> Self {
        let mut brush = BrushEngine::new();

        // Source env.sh in the embedded brush engine so bash functions
        // (like nvm) persist across commands in bash mode.
        let config_dir = crate::shell::config_dir();
        let env_sh = config_dir.join("env.sh");
        if env_sh.exists() {
            let state = ShellState::from_current_env();
            brush.inject_state(&state);
            let source_cmd = format!(". '{}'", env_sh.display());
            brush.execute(&source_cmd);
        }

        ShannonDispatcher { brush }
    }

    /// Get the current env vars from brush (after sourcing env.sh).
    /// Used to inject bash env vars into nushell's Stack at startup.
    pub fn env_vars(&self) -> HashMap<String, String> {
        self.brush.capture_env()
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
                self.brush.inject_state(&state);
                let result = self.brush.execute(command);
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
