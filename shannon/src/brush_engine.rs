use std::collections::HashMap;
use std::collections::HashSet;

use brush_builtins::ShellBuilderExt;
use brush_core::{ExecutionExitCode, Shell, ShellVariable};

use crate::shell::ShellState;

pub struct BrushEngine {
    shell: Shell,
    runtime: tokio::runtime::Runtime,
    /// Track all env var names we know about (from inject + commands).
    known_keys: HashSet<String>,
}

impl BrushEngine {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime for brush");

        let shell = runtime
            .block_on(
                Shell::builder()
                    .default_builtins(brush_builtins::BuiltinSet::BashMode)
                    .build(),
            )
            .expect("failed to create brush shell");

        // Seed known keys from process env (brush inherits these)
        let known_keys: HashSet<String> = std::env::vars().map(|(k, _)| k).collect();

        BrushEngine {
            shell,
            runtime,
            known_keys,
        }
    }

    /// Inject shannon's ShellState into the brush shell before evaluation.
    pub fn inject_state(&mut self, state: &ShellState) {
        // Set cwd
        let _ = self.shell.set_working_dir(&state.cwd);

        // Inject env vars as exported variables
        for (key, value) in &state.env {
            let mut var = ShellVariable::new(value.as_str());
            var.export();
            let _ = self.shell.set_env_global(key, var);
            self.known_keys.insert(key.clone());
        }
    }

    /// Execute a bash command natively and return updated state.
    pub fn execute(&mut self, command: &str) -> ShellState {
        let params = self.shell.default_exec_params();

        let result = self
            .runtime
            .block_on(self.shell.run_string(command, &params));

        let exit_code = match result {
            Ok(r) => self.exit_code_to_i32(&r.exit_code),
            Err(_) => 1,
        };

        // Discover newly exported vars by checking the command for export/declare
        self.discover_new_keys(command);

        let env = self.capture_env();
        let cwd = self.shell.working_dir().to_path_buf();

        ShellState {
            env,
            cwd,
            last_exit_code: exit_code,
        }
    }

    fn exit_code_to_i32(&self, code: &ExecutionExitCode) -> i32 {
        match code {
            ExecutionExitCode::Success => 0,
            ExecutionExitCode::GeneralError => 1,
            ExecutionExitCode::InvalidUsage => 2,
            ExecutionExitCode::CannotExecute => 126,
            ExecutionExitCode::NotFound => 127,
            ExecutionExitCode::Interrupted => 130,
            ExecutionExitCode::Unimplemented => 99,
            ExecutionExitCode::Custom(c) => *c as i32,
        }
    }

    /// Try to discover env var names from the command text.
    /// This is a heuristic — catches `export FOO=bar` patterns.
    fn discover_new_keys(&mut self, command: &str) {
        for word in command.split_whitespace() {
            if let Some(eq_pos) = word.find('=') {
                let key = &word[..eq_pos];
                if !key.is_empty() && key.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    self.known_keys.insert(key.to_string());
                }
            }
        }
    }

    /// Capture env vars by querying brush for all known keys.
    fn capture_env(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        for key in &self.known_keys {
            if let Some(var) = self.shell.env_var(key) {
                if var.is_exported() {
                    if let Some(val) = self.shell.env_str(key) {
                        env.insert(key.clone(), val.to_string());
                    }
                }
            }
        }

        env
    }
}
