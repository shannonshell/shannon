use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use nu_cli::{ModeDispatcher, ModeResult};
use signal_hook::SigId;

fn debug_log(msg: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/shannon-debug.log")
    {
        let _ = writeln!(f, "{msg}");
    }
}

use crate::brush_engine::BrushEngine;
use crate::shell::ShellState;
use crate::shell_engine::ShellEngine;

/// Holds nushell's SIGINT handler registration so it can be temporarily
/// unregistered during brush execution.
struct SigintGuard {
    sig_id: SigId,
    handler: Arc<dyn Fn() + Send + Sync>,
}

/// Drop guard that re-registers nushell's SIGINT handler when dropped.
/// Ensures re-registration even if brush execution panics.
struct ReregisterOnDrop<'a> {
    guard: &'a mut SigintGuard,
}

impl<'a> Drop for ReregisterOnDrop<'a> {
    fn drop(&mut self) {
        let handler_clone = self.guard.handler.clone();
        let new_id = unsafe {
            signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
                handler_clone();
            })
        }
        .expect("Failed to re-register SIGINT handler after brush execution");
        self.guard.sig_id = new_id;
    }
}

pub struct ShannonDispatcher {
    brush: BrushEngine,
    sigint: Option<SigintGuard>,
}

impl ShannonDispatcher {
    pub fn new(
        sigint_handler: Option<(SigId, Arc<dyn Fn() + Send + Sync>)>,
    ) -> Self {
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

        let sigint = sigint_handler.map(|(sig_id, handler)| SigintGuard { sig_id, handler });

        ShannonDispatcher { brush, sigint }
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
                // Temporarily unregister nushell's SIGINT handler so brush's
                // tokio::signal::ctrl_c() can receive SIGINT for child processes.
                let _reregister = if let Some(ref mut sigint) = self.sigint {
                    signal_hook::low_level::unregister(sigint.sig_id);
                    Some(ReregisterOnDrop { guard: sigint })
                } else {
                    None
                };

                debug_log(&format!("[shannon:dispatcher] entering brush execute: {command}"));
                self.brush.inject_state(&state);
                let result = self.brush.execute(command);
                debug_log("[shannon:dispatcher] brush execute complete");
                // _reregister drops here, re-registering the handler

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
