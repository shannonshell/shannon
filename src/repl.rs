use std::io;
use std::process::Command;

use chrono::Utc;
use crossterm::event::{KeyCode, KeyModifiers};
use nu_ansi_term::{Color, Style};
use reedline::{
    default_vi_insert_keybindings, default_vi_normal_keybindings, ColumnarMenu, DefaultHinter,
    EditCommand, HistorySessionId, MenuBuilder, Reedline, ReedlineEvent, ReedlineMenu, Signal,
    SqliteBackedHistory, Vi,
};

use crate::completer::ShannonCompleter;
use crate::config::ShellConfig;
use crate::executor::execute_command;
use crate::highlighter::TreeSitterHighlighter;
use crate::prompt::ShannonPrompt;
use crate::shell::{self, ShellState};

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
) -> Reedline {
    let mut insert_keybindings = default_vi_insert_keybindings();
    let mut normal_keybindings = default_vi_normal_keybindings();

    // Add shannon-specific bindings to both modes
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

    // Right arrow accepts hint in insert mode only
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

    let highlighter = TreeSitterHighlighter::new(shell_config.highlighter.as_deref());

    let hinter = DefaultHinter::default()
        .with_style(Style::new().fg(Color::Rgb(86, 95, 137))); // Tokyo Night muted #565f89

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

/// Run the main read-eval-print loop.
pub fn run(
    shells: Vec<(String, ShellConfig)>,
    mut state: ShellState,
    depth: u32,
) -> io::Result<()> {
    let session_id = Reedline::create_history_session_id();

    let mut active_idx = 0;
    let mut editor = build_editor(&shells[active_idx].1, session_id);

    loop {
        let prompt = ShannonPrompt {
            shell_name: shells[active_idx].0.clone(),
            cwd: state.cwd.clone(),
            last_exit_code: state.last_exit_code,
            depth,
        };

        match editor.read_line(&prompt) {
            Ok(Signal::Success(line)) => {
                if line == SWITCH_COMMAND {
                    active_idx = (active_idx + 1) % shells.len();
                    editor = build_editor(&shells[active_idx].1, session_id);
                    continue;
                }

                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                if line == "exit" {
                    break;
                }

                // Update history entry with timestamp and cwd
                let cwd = state.cwd.to_string_lossy().to_string();
                let _ = editor.update_last_command_context(&|mut c| {
                    c.start_timestamp = Some(Utc::now());
                    c.cwd = Some(cwd.clone());
                    c
                });

                match execute_command(&shells[active_idx].1, line, &state) {
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
