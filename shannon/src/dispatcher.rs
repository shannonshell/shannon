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
        ShannonDispatcher {
            brush: BrushEngine::new(),
        }
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
            "brush" => {
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
