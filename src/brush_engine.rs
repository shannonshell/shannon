use std::collections::HashMap;

use brush_builtins::ShellBuilderExt;
use brush_core::{ExecutionExitCode, Shell, ShellValue, ShellVariable, SourceInfo};

use crate::shell::ShellState;
use crate::shell_engine::ShellEngine;

pub struct BrushEngine {
    shell: Shell,
    runtime: tokio::runtime::Runtime,
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

        BrushEngine { shell, runtime }
    }

    fn do_inject_state(&mut self, state: &ShellState) {
        // Set cwd
        let _ = self.shell.set_working_dir(&state.cwd);

        // Inject env vars as exported variables
        for (key, value) in &state.env {
            let mut var = ShellVariable::new(value.as_str());
            var.export();
            let _ = self.shell.env_mut().set_global(key.clone(), var);
        }
    }

    fn do_execute(&mut self, command: &str) -> ShellState {
        let params = self.shell.default_exec_params();

        let result = self.runtime.block_on(self.shell.run_string(
            command,
            &SourceInfo::default(),
            &params,
        ));

        let exit_code = match result {
            Ok(r) => self.exit_code_to_i32(&r.exit_code),
            Err(_) => 1,
        };

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
            ExecutionExitCode::BrokenPipe => 141, // 128 + SIGPIPE(13)
            ExecutionExitCode::Custom(c) => *c as i32,
        }
    }

    /// Capture all exported env vars directly from brush's environment.
    pub fn capture_env(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        for (name, var) in self.shell.env().iter_exported() {
            if let ShellValue::String(s) = var.value() {
                env.insert(name.clone(), s.clone());
            }
        }

        env
    }
}

impl ShellEngine for BrushEngine {
    fn inject_state(&mut self, state: &ShellState) {
        self.do_inject_state(state);
    }

    fn execute(&mut self, command: &str) -> ShellState {
        self.do_execute(command)
    }
}
