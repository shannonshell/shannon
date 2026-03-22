use std::collections::HashMap;
use std::path::PathBuf;

use nu_cli::eval_source;
use nu_protocol::engine::{EngineState, Stack};
use nu_protocol::{PipelineData, Span, Value};

use crate::shell::ShellState;

pub struct NushellEngine {
    engine_state: EngineState,
    stack: Stack,
}

impl NushellEngine {
    pub fn new() -> Self {
        // Initialize engine with all built-in commands
        let engine_state = EngineState::new();
        let engine_state = nu_cmd_lang::add_default_context(engine_state);
        let engine_state = nu_command::add_shell_command_context(engine_state);
        let engine_state = nu_cli::add_cli_context(engine_state);

        let stack = Stack::new();
        NushellEngine {
            engine_state,
            stack,
        }
    }

    /// Inject shannon's ShellState into the nushell engine before evaluation.
    pub fn inject_state(&mut self, state: &ShellState) {
        // Set cwd
        let _ = self.stack.set_cwd(&state.cwd);

        // Inject env vars
        for (key, value) in &state.env {
            self.stack.add_env_var(
                key.clone(),
                Value::string(value.clone(), Span::unknown()),
            );
        }
    }

    /// Execute a nushell command natively and return updated state.
    pub fn execute(&mut self, command: &str) -> ShellState {
        let exit_code = eval_source(
            &mut self.engine_state,
            &mut self.stack,
            command.as_bytes(),
            "shannon",
            PipelineData::empty(),
            false,
        );

        self.capture_state(exit_code)
    }

    /// Read current state from the nushell Stack.
    fn capture_state(&self, exit_code: i32) -> ShellState {
        let nu_env = self.stack.get_env_vars(&self.engine_state);
        let mut env = HashMap::new();

        for (key, value) in nu_env {
            if let Ok(s) = value.as_str() {
                env.insert(key, s.to_string());
            } else if let Ok(list) = value.clone().into_list() {
                // Join lists (like PATH) with ':'
                let parts: Vec<String> = list
                    .iter()
                    .filter_map(|v| v.as_str().ok().map(|s| s.to_string()))
                    .collect();
                if !parts.is_empty() {
                    let sep = if cfg!(windows) { ";" } else { ":" };
                    env.insert(key, parts.join(sep));
                }
            }
            // Skip non-string, non-list values (records, closures, etc.)
        }

        let cwd = self
            .stack
            .get_env_var(&self.engine_state, "PWD")
            .and_then(|v| v.as_str().ok())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/"));

        ShellState {
            env,
            cwd,
            last_exit_code: exit_code,
        }
    }
}
