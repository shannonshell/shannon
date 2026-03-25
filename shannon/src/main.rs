use std::io;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use shannonshell::ai_engine::AiEngine;
use shannonshell::brush_engine::BrushEngine;
use shannonshell::config::ShannonConfig;
use shannonshell::executor::run_startup_script;
use shannonshell::nushell_engine::NushellEngine;
use shannonshell::repl;
use shannonshell::shell::ShellState;
use shannonshell::shell_engine::ShellSlot;
use shannonshell::theme::Theme;

fn main() -> io::Result<()> {
    // Load config.toml (or use built-in defaults)
    let config = ShannonConfig::load();

    // Run env script first so PATH is complete
    let mut state = run_startup_script(ShellState::from_current_env());

    // Track nesting depth for nested shannon instances
    let depth: u32 = state
        .env
        .get("SHANNON_DEPTH")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
        + 1;
    state
        .env
        .insert("SHANNON_DEPTH".to_string(), depth.to_string());
    std::env::set_var("SHANNON_DEPTH", depth.to_string());

    // Update process PATH
    if let Some(path) = state.env.get("PATH") {
        std::env::set_var("PATH", path);
    }

    // Shared interrupt flag for signal handling.
    let interrupt = Arc::new(AtomicBool::new(false));

    // Build shell engines keyed by name
    let mut engines: Vec<(&str, Box<dyn shannonshell::shell_engine::ShellEngine>)> = vec![
        ("nu", Box::new(NushellEngine::new(interrupt.clone()))),
        ("brush", Box::new(BrushEngine::new())),
        ("ai", Box::new(AiEngine::new(config.ai.clone()))),
    ];

    // Get ordered shell names from config
    let shell_order = config.shell_order();

    // Build ShellSlot list in config order
    let mut shells: Vec<ShellSlot> = Vec::new();
    for name in &shell_order {
        if let Some(pos) = engines.iter().position(|(n, _)| *n == name.as_str()) {
            let (_, engine) = engines.remove(pos);
            shells.push(ShellSlot {
                name: name.clone(),
                highlighter: ShannonConfig::highlighter_for(name),
                engine,
            });
        }
    }

    if shells.is_empty() {
        eprintln!("shannon: no supported shells found");
        std::process::exit(1);
    }

    // Build theme from config
    let theme = Theme::from_config(&config.theme);

    // Run the REPL
    repl::run(shells, state, depth, theme, interrupt)
}
