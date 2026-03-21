use std::io;
use std::process::Command;

use crossterm::event::{KeyCode, KeyModifiers};
use reedline::{
    default_emacs_keybindings, Emacs, FileBackedHistory, Reedline, ReedlineEvent, Signal,
};

use shannon::executor::execute_command;
use shannon::highlighter::TreeSitterHighlighter;
use shannon::prompt::ShannonPrompt;
use shannon::shell::{ShellKind, ShellState};

const SWITCH_COMMAND: &str = "__shannon_switch";

fn shell_available(shell: ShellKind) -> bool {
    Command::new(shell.binary())
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

fn build_editor(shell: ShellKind) -> Reedline {
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::SHIFT,
        KeyCode::BackTab,
        ReedlineEvent::ExecuteHostCommand(SWITCH_COMMAND.into()),
    );
    let edit_mode = Box::new(Emacs::new(keybindings));

    let history_file = shell.history_file();
    if let Some(parent) = history_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let history =
        FileBackedHistory::with_file(10000, history_file).expect("failed to create history file");

    let highlighter = TreeSitterHighlighter::new(shell);

    Reedline::create()
        .with_edit_mode(edit_mode)
        .with_history(Box::new(history))
        .with_highlighter(Box::new(highlighter))
}

fn main() -> io::Result<()> {
    // Detect available shells
    let shells: Vec<ShellKind> = [ShellKind::Bash, ShellKind::Nushell]
        .into_iter()
        .filter(|s| shell_available(*s))
        .collect();

    if shells.is_empty() {
        eprintln!("shannon: no supported shells found (looked for bash, nu)");
        std::process::exit(1);
    }

    let mut active_shell = shells[0];
    let mut state = ShellState::from_current_env();
    let mut editor = build_editor(active_shell);

    loop {
        let prompt = ShannonPrompt {
            shell: active_shell,
            cwd: state.cwd.clone(),
            last_exit_code: state.last_exit_code,
        };

        match editor.read_line(&prompt) {
            Ok(Signal::Success(line)) => {
                if line == SWITCH_COMMAND {
                    // Cycle to next available shell
                    let current_idx = shells.iter().position(|s| *s == active_shell).unwrap_or(0);
                    active_shell = shells[(current_idx + 1) % shells.len()];
                    editor = build_editor(active_shell);
                    continue;
                }

                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                if line == "exit" {
                    break;
                }

                match execute_command(active_shell, line, &state) {
                    Ok(new_state) => {
                        state = new_state;
                    }
                    Err(e) => {
                        eprintln!("shannon: {e}");
                        state.last_exit_code = 1;
                    }
                }
            }
            Ok(Signal::CtrlD) => break,
            Ok(Signal::CtrlC) => continue,
            Err(e) => {
                eprintln!("shannon: {e}");
                break;
            }
        }
    }

    Ok(())
}
