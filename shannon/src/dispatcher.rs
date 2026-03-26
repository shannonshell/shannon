use std::collections::HashMap;
use std::path::PathBuf;

use nu_cli::{ModeDispatcher, ModeResult};

use crate::brush_engine::BrushEngine;
use crate::ai_engine::AiEngine;
use crate::shell::ShellState;
use crate::shell_engine::ShellEngine;

pub struct ShannonDispatcher {
    brush: BrushEngine,
    ai: AiEngine,
}

impl ShannonDispatcher {
    pub fn new() -> Self {
        // Default AI config from env vars
        let ai_config = crate::ai_engine::AiConfig::default();
        ShannonDispatcher {
            brush: BrushEngine::new(),
            ai: AiEngine::new(ai_config),
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
            "ai" => {
                self.ai.inject_state(&state);
                self.ai.execute(command);
                ModeResult {
                    env: state.env,
                    cwd: state.cwd,
                    exit_code: 0,
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
