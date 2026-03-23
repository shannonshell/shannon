use std::io;
use std::io::Write;
use std::process::Command;

use chrono::Utc;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal;
use nu_ansi_term::{Color, Style};
use reedline::{
    default_vi_insert_keybindings, default_vi_normal_keybindings, ColumnarMenu, DefaultHinter,
    EditCommand, HistorySessionId, MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, Signal,
    SqliteBackedHistory, Vi,
};

use crate::ai::session::Session;
use crate::ai::translate::translate_command;
use crate::completer::ShannonCompleter;
use crate::config::{AiConfig, ShellConfig};
use crate::executor::execute_command;
use crate::highlighter::TreeSitterHighlighter;
use crate::nushell_engine::NushellEngine;
use crate::prompt::{tilde_contract, ShannonPrompt};
use crate::shell::{self, ShellState};
use crate::theme::Theme;

const SWITCH_COMMAND: &str = "__shannon_switch";

pub fn shell_available(binary: &str) -> bool {
    Command::new(binary)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

fn build_editor(
    shell_config: &ShellConfig,
    session_id: Option<HistorySessionId>,
    ai_mode: bool,
    theme: &Theme,
) -> Reedline {
    let mut insert_keybindings = default_vi_insert_keybindings();
    let mut normal_keybindings = default_vi_normal_keybindings();

    for kb in [&mut insert_keybindings, &mut normal_keybindings] {
        kb.add_binding(
            KeyModifiers::SHIFT,
            KeyCode::BackTab,
            ReedlineEvent::ExecuteHostCommand(SWITCH_COMMAND.into()),
        );
        kb.add_binding(
            KeyModifiers::NONE,
            KeyCode::Tab,
            ReedlineEvent::UntilFound(vec![
                ReedlineEvent::Menu("completion_menu".to_string()),
                ReedlineEvent::MenuNext,
            ]),
        );
    }

    insert_keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Right,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::HistoryHintComplete,
            ReedlineEvent::Edit(vec![EditCommand::MoveRight { select: false }]),
        ]),
    );

    let edit_mode = Box::new(Vi::new(insert_keybindings, normal_keybindings));

    let history_db = shell::history_db();
    let history = SqliteBackedHistory::with_file(history_db, session_id, Some(Utc::now()))
        .expect("failed to create history database");

    let highlighter = if ai_mode {
        TreeSitterHighlighter::new(None, theme)
    } else {
        TreeSitterHighlighter::new(shell_config.highlighter.as_deref(), theme)
    };

    let hinter = DefaultHinter::default().with_style(theme.hint);

    let completer = Box::new(ShannonCompleter::new());
    let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));

    Reedline::create()
        .with_edit_mode(edit_mode)
        .with_history(Box::new(history))
        .with_history_session_id(session_id)
        .with_highlighter(Box::new(highlighter))
        .with_hinter(Box::new(hinter))
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .use_bracketed_paste(true)
}

/// Read a single keypress from the terminal (Enter or Esc).
fn read_confirmation() -> io::Result<KeyCode> {
    terminal::enable_raw_mode()?;
    let result = loop {
        if let Event::Key(key_event) = event::read()? {
            match key_event.code {
                KeyCode::Enter | KeyCode::Esc | KeyCode::Char('e') => {
                    break Ok(key_event.code);
                }
                _ => continue,
            }
        }
    };
    terminal::disable_raw_mode()?;
    result
}

/// Emit OSC 7 to report the current working directory to the terminal.
fn emit_osc7(cwd: &std::path::Path) {
    let hostname = std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string());
    let path = cwd.to_string_lossy();
    let encoded: String = path
        .chars()
        .map(|c| {
            if c.is_ascii_control() || c == ' ' {
                format!("%{:02X}", c as u32)
            } else {
                c.to_string()
            }
        })
        .collect();
    eprint!(
        "\x1b]7;file://{}{}{}\x1b\\",
        hostname,
        if encoded.starts_with('/') { "" } else { "/" },
        encoded
    );
}

/// Emit OSC 2 to set the terminal title (idle — showing shell + cwd).
fn emit_osc2_idle(shell_name: &str, cwd: &std::path::PathBuf) {
    let path = tilde_contract(cwd);
    eprint!("\x1b]2;[{}] {}\x07", shell_name, path);
}

/// Emit OSC 2 to set the terminal title (running a command).
fn emit_osc2_command(shell_name: &str, cwd: &std::path::PathBuf, command: &str) {
    let path = tilde_contract(cwd);
    let binary = command.split_whitespace().next().unwrap_or(command);
    eprint!("\x1b]2;[{}] {}> {}\x07", shell_name, path, binary);
}

/// Execute a command using the nushell engine (for "nu") or wrapper (for others).
fn run_command(
    shell: &(String, ShellConfig),
    command: &str,
    state: &mut ShellState,
    nushell_engine: &mut Option<NushellEngine>,
) {
    if shell.0 == "nu" {
        if let Some(ref mut engine) = nushell_engine {
            engine.inject_state(state);
            *state = engine.execute(command);
            return;
        }
    }
    // Wrapper path for bash/fish/zsh (and fallback for nu without engine)
    match execute_command(&shell.1, command, state) {
        Ok(new_state) => {
            *state = new_state;
        }
        Err(e) => {
            eprintln!("shannon: {e}");
            state.last_exit_code = 1;
        }
    }
}

/// Run the main read-eval-print loop.
pub fn run(
    shells: Vec<(String, ShellConfig)>,
    ai_config: AiConfig,
    mut state: ShellState,
    depth: u32,
    mut nushell_engine: Option<NushellEngine>,
    theme: Theme,
) -> io::Result<()> {
    let session_id = Reedline::create_history_session_id();

    let mut active_idx = 0;
    let mut ai_mode = false;
    let mut editor = build_editor(&shells[active_idx].1, session_id, ai_mode, &theme);
    let mut ai_session: Option<Session> = None;

    loop {
        // Report cwd and title to terminal
        emit_osc7(&state.cwd);
        emit_osc2_idle(&shells[active_idx].0, &state.cwd);

        let prompt = ShannonPrompt {
            shell_name: shells[active_idx].0.clone(),
            cwd: state.cwd.clone(),
            last_exit_code: state.last_exit_code,
            depth,
            ai_mode,
            prompt_color: theme.prompt,
            indicator_color: theme.prompt_indicator,
            error_color: theme.prompt_error,
        };

        match editor.read_line(&prompt) {
            Ok(Signal::Success(line)) => {
                if line == SWITCH_COMMAND {
                    active_idx = (active_idx + 1) % shells.len();
                    editor = build_editor(&shells[active_idx].1, session_id, ai_mode, &theme);
                    continue;
                }

                let line = line.trim();

                // Empty line toggles AI mode
                if line.is_empty() {
                    if ai_mode {
                        ai_mode = false;
                        ai_session = None;
                    } else {
                        ai_mode = true;
                        ai_session = Some(Session::new());
                    }
                    // Rebuild editor to toggle highlighting
                    editor = build_editor(&shells[active_idx].1, session_id, ai_mode, &theme);
                    continue;
                }

                if line == "exit" {
                    break;
                }

                if ai_mode {
                    // AI mode: translate natural language to command
                    eprint!("  Thinking...");
                    io::stderr().flush().ok();

                    let session = ai_session.as_mut().unwrap();
                    let cwd = state.cwd.to_string_lossy().to_string();
                    let shell_name = &shells[active_idx].0;

                    match translate_command(&ai_config, session, shell_name, &cwd, line) {
                        Ok(command) => {
                            // Clear "Thinking..." and show the command
                            eprint!("\r\x1b[K");
                            eprintln!("  \x1b[36m→\x1b[0m {command}");
                            eprintln!("  \x1b[90m[Enter] run  [Esc] cancel\x1b[0m");

                            match read_confirmation()? {
                                KeyCode::Enter => {
                                    eprintln!(); // newline after confirmation
                                    // Run the command through the active shell
                                    emit_osc2_command(&shells[active_idx].0, &state.cwd, &command);
                                    run_command(
                                        &shells[active_idx],
                                        &command,
                                        &mut state,
                                        &mut nushell_engine,
                                    );
                                    emit_osc7(&state.cwd);

                                    // Exit AI mode after execution
                                    ai_mode = false;
                                    ai_session = None;
                                    editor = build_editor(
                                        &shells[active_idx].1,
                                        session_id,
                                        ai_mode,
                                        &theme,
                                    );
                                }
                                KeyCode::Esc => {
                                    // Cancel — stay in AI mode
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            eprint!("\r\x1b[K");
                            eprintln!("  shannon: {e}");
                        }
                    }
                } else {
                    // Normal mode: execute command directly
                    let cwd = state.cwd.to_string_lossy().to_string();
                    let _ = editor.update_last_command_context(&|mut c| {
                        c.start_timestamp = Some(Utc::now());
                        c.cwd = Some(cwd.clone());
                        c
                    });

                    emit_osc2_command(&shells[active_idx].0, &state.cwd, line);
                    run_command(
                        &shells[active_idx],
                        line,
                        &mut state,
                        &mut nushell_engine,
                    );
                    emit_osc7(&state.cwd);
                }
            }
            Ok(Signal::CtrlD) => break,
            Ok(Signal::CtrlC) => {
                if ai_mode {
                    ai_mode = false;
                    ai_session = None;
                }
                continue;
            }
            Err(e) => {
                eprintln!("shannon: {e}");
                break;
            }
        }
    }

    Ok(())
}
