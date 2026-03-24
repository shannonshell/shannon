use std::io;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use shannonshell::brush_engine::BrushEngine;
use shannonshell::config::ShannonConfig;
use shannonshell::executor::run_startup_script;
use shannonshell::nushell_engine::NushellEngine;
use shannonshell::repl;
use shannonshell::shell::ShellState;
use shannonshell::theme::Theme;

fn main() -> io::Result<()> {
    // Load config.toml (or use built-in defaults)
    let config = ShannonConfig::load();

    // Run env script first so PATH is complete before shell detection
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

    // Update process PATH so shell_available() can find binaries
    if let Some(path) = state.env.get("PATH") {
        std::env::set_var("PATH", path);
    }

    // Get ordered shell list from config, filter to installed shells
    let all_shells = config.shells();
    let shells = all_shells
        .into_iter()
        .filter(|(name, cfg)| name == "nu" || name == "brush" || repl::shell_available(&cfg.binary))
        .collect::<Vec<_>>();

    if shells.is_empty() {
        eprintln!("shannon: no supported shells found");
        std::process::exit(1);
    }

    // Shared interrupt flag for nushell's signal system.
    // signal-hook registers a SIGINT handler that sets this flag.
    // Nushell checks it via signals.interrupted() during execution.
    let interrupt = Arc::new(AtomicBool::new(false));

    // Initialize embedded engines (always available)
    let nushell_engine = Some(NushellEngine::new(interrupt.clone()));
    let brush_engine = Some(BrushEngine::new());

    // Build theme from config
    let theme = Theme::from_config(&config.theme);

    // Run the REPL
    repl::run(
        shells, config.ai, state, depth, nushell_engine, brush_engine, theme, interrupt,
    )
}
