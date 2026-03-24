use std::io;
use std::io::Write;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use chrono::Utc;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal;
use reedline::{
    default_vi_insert_keybindings, default_vi_normal_keybindings, ColumnarMenu, Completer,
    DefaultHinter, EditCommand, HistorySessionId, MenuBuilder, Reedline, ReedlineEvent,
    ReedlineMenu, Signal, Span, SqliteBackedHistory, Suggestion, Vi,
};

use crate::ai::session::Session;
use crate::ai::translate::translate_command;
use crate::brush_engine::BrushEngine;
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

/// Completer that returns available shells for the Ctrl+Tab picker menu.
struct ShellSwitchCompleter {
    shells: Vec<String>,
}

impl Completer for ShellSwitchCompleter {
    fn complete(&mut self, _line: &str, _pos: usize) -> Vec<Suggestion> {
        self.shells
            .iter()
            .map(|name| Suggestion {
                value: format!("/switch {name}"),
                display_override: Some(name.clone()),
                description: None,
                style: None,
                extra: None,
                span: Span::new(0, 0),
                append_whitespace: false,
                match_indices: None,
            })
            .collect()
    }
}

fn build_editor(
    shell_config: &ShellConfig,
    session_id: Option<HistorySessionId>,
    ai_mode: bool,
    theme: &Theme,
    shell_names: &[String],
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
        kb.add_binding(
            KeyModifiers::CONTROL,
            KeyCode::Char('s'),
            ReedlineEvent::Menu("shell_menu".to_string()),
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

    let shell_menu = ReedlineMenu::WithCompleter {
        menu: Box::new(
            reedline::IdeMenu::default()
                .with_name("shell_menu")
                .with_default_border(),
        ),
        completer: Box::new(ShellSwitchCompleter {
            shells: shell_names.to_vec(),
        }),
    };

    Reedline::create()
        .with_edit_mode(edit_mode)
        .with_history(Box::new(history))
        .with_history_session_id(session_id)
        .with_highlighter(Box::new(highlighter))
        .with_hinter(Box::new(hinter))
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_menu(shell_menu)
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

/// Handle a `/` meta-command. Returns true if handled, false if the shell should run it.
fn handle_meta_command(
    line: &str,
    shells: &[(String, ShellConfig)],
    active_idx: &mut usize,
    editor: &mut Reedline,
    session_id: Option<HistorySessionId>,
    ai_mode: bool,
    theme: &Theme,
    shell_names: &[String],
) -> bool {
    let parts: Vec<&str> = line.splitn(2, ' ').collect();
    let cmd = parts[0];
    let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

    // If a file with this path exists, let the shell handle it
    if std::path::Path::new(cmd).exists() {
        return false;
    }

    match cmd {
        "/switch" => {
            if let Some(idx) = shells.iter().position(|(n, _)| n == arg) {
                *active_idx = idx;
                *editor = build_editor(
                    &shells[*active_idx].1,
                    session_id,
                    ai_mode,
                    theme,
                    shell_names,
                );
            } else if !arg.is_empty() {
                eprintln!("shannon: unknown shell: {arg}");
            } else {
                let names: Vec<&str> = shells.iter().map(|(n, _)| n.as_str()).collect();
                eprintln!("Available shells: {}", names.join(", "));
            }
            true
        }
        "/help" => {
            eprintln!("Shannon commands:");
            eprintln!("  /switch <shell>  — switch to a shell");
            eprintln!("  /help            — show this help");
            eprintln!("  Shift+Tab        — cycle to next shell");
            eprintln!("  Ctrl+S           — shell picker menu");
            eprintln!("  Enter (empty)    — toggle AI mode");
            true
        }
        _ => false,
    }
}

/// Re-register signal-hook's SIGINT handler.
///
/// Called after wrapper execution (bash/fish/zsh) because executor.rs sets
/// SIG_IGN which overwrites signal-hook's handler. This restores it so
/// nushell's Signals system works on the next nushell command.
#[cfg(unix)]
fn restore_sigint_handler(interrupt: &Arc<AtomicBool>) {
    signal_hook::flag::register(signal_hook::consts::SIGINT, interrupt.clone())
        .expect("failed to re-register SIGINT handler");
}

/// Execute a command using an embedded engine or wrapper.
fn run_command(
    shell: &(String, ShellConfig),
    command: &str,
    state: &mut ShellState,
    nushell_engine: &mut Option<NushellEngine>,
    brush_engine: &mut Option<BrushEngine>,
    interrupt: &Arc<AtomicBool>,
) {
    if shell.0 == "nu" {
        if let Some(ref mut engine) = nushell_engine {
            // Nushell's Signals system handles SIGINT internally.
            // signal-hook sets the interrupt AtomicBool, nushell checks it during execution.
            engine.inject_state(state);
            *state = engine.execute(command);
            return;
        }
    }
    if shell.0 == "brush" {
        if let Some(ref mut engine) = brush_engine {
            engine.inject_state(state);
            *state = engine.execute(command);
            return;
        }
    }
    // Wrapper path for bash/fish/zsh (and fallback without engine)
    match execute_command(&shell.1, command, state) {
        Ok(new_state) => {
            *state = new_state;
        }
        Err(e) => {
            eprintln!("shannon: {e}");
            state.last_exit_code = 1;
        }
    }
    // executor.rs sets SIG_IGN which overwrites signal-hook's handler.
    // Re-register it so nushell's signal system works next time.
    #[cfg(unix)]
    restore_sigint_handler(interrupt);
}

/// Run the main read-eval-print loop.
pub fn run(
    shells: Vec<(String, ShellConfig)>,
    ai_config: AiConfig,
    mut state: ShellState,
    depth: u32,
    mut nushell_engine: Option<NushellEngine>,
    mut brush_engine: Option<BrushEngine>,
    theme: Theme,
    interrupt: Arc<AtomicBool>,
) -> io::Result<()> {
    let session_id = Reedline::create_history_session_id();
    let shell_names: Vec<String> = shells.iter().map(|(n, _)| n.clone()).collect();

    // Register signal-hook's SIGINT handler. This replaces SIG_DFL with a
    // handler that sets the AtomicBool. Shannon won't die from SIGINT.
    // Reedline uses crossterm raw mode — Ctrl+C is a keypress, not a signal.
    #[cfg(unix)]
    signal_hook::flag::register(signal_hook::consts::SIGINT, interrupt.clone())
        .expect("failed to register SIGINT handler");

    let mut active_idx = 0;
    let mut ai_mode = false;
    let mut editor = build_editor(&shells[active_idx].1, session_id, ai_mode, &theme, &shell_names);
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
            ai_badge_style: theme.ai_badge,
        };

        match editor.read_line(&prompt) {
            Ok(Signal::Success(line)) => {
                // Shift+Tab: cycle to next shell
                if line == SWITCH_COMMAND {
                    active_idx = (active_idx + 1) % shells.len();
                    editor = build_editor(&shells[active_idx].1, session_id, ai_mode, &theme, &shell_names);
                    continue;
                }

                let line = line.trim();

                // Meta-commands: /switch, /help, etc.
                if line.starts_with('/') {
                    if handle_meta_command(
                        line,
                        &shells,
                        &mut active_idx,
                        &mut editor,
                        session_id,
                        ai_mode,
                        &theme,
                        &shell_names,
                    ) {
                        continue;
                    }
                    // Not a known meta-command — fall through to shell
                }

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
                    editor = build_editor(&shells[active_idx].1, session_id, ai_mode, &theme, &shell_names);
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
                                        &mut brush_engine,
                                        &interrupt,
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
                                        &shell_names,
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
                        &mut brush_engine,
                        &interrupt,
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
                state.last_exit_code = 0;
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
