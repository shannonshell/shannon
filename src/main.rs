use std::io;

use shannon::config::ShannonConfig;
use shannon::executor::run_startup_script;
use shannon::nushell_engine::NushellEngine;
use shannon::repl;
use shannon::shell::ShellState;

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
        .filter(|(name, cfg)| name == "nu" || repl::shell_available(&cfg.binary))
        .collect::<Vec<_>>();

    if shells.is_empty() {
        eprintln!("shannon: no supported shells found");
        std::process::exit(1);
    }

    // Initialize nushell native engine (always embedded)
    let nushell_engine = Some(NushellEngine::new());

    // Run the REPL
    repl::run(shells, config.ai, state, depth, nushell_engine)
}
