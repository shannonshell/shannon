use std::path::PathBuf;

use shannonshell::brush_engine::BrushEngine;
use shannonshell::nushell_engine::NushellEngine;
use shannonshell::shell::ShellState;
use shannonshell::shell_engine::ShellEngine;

fn initial_state() -> ShellState {
    ShellState::from_current_env()
}

// --- Nushell tests (embedded via NushellEngine) ---

#[test]
fn test_nushell_echo() {
    let mut engine = NushellEngine::new(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)));
    engine.inject_state(&initial_state());
    let result = engine.execute("echo hello");
    assert_eq!(result.last_exit_code, 0);
}

#[test]
fn test_nushell_env_capture() {
    let mut engine = NushellEngine::new(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)));
    engine.inject_state(&initial_state());
    let result = engine.execute(r#"$env.FOO = "test_value_456""#);
    assert_eq!(result.env.get("FOO").unwrap(), "test_value_456");
}

#[test]
fn test_nushell_cwd_capture() {
    let mut engine = NushellEngine::new(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)));
    engine.inject_state(&initial_state());
    let result = engine.execute("cd /tmp");
    assert!(
        result.cwd == PathBuf::from("/tmp") || result.cwd == PathBuf::from("/private/tmp"),
        "unexpected cwd: {:?}",
        result.cwd
    );
}

#[test]
fn test_nushell_state_persistence() {
    let mut engine = NushellEngine::new(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)));
    engine.inject_state(&initial_state());
    let state1 = engine.execute(r#"$env.PERSIST = "hello""#);
    assert_eq!(state1.env.get("PERSIST").unwrap(), "hello");
    // Engine persists state across commands (no re-inject needed)
    let state2 = engine.execute("$env.PERSIST");
    assert_eq!(state2.last_exit_code, 0);
}

// --- Brush tests (embedded bash via BrushEngine) ---

#[test]
fn test_brush_echo() {
    let mut engine = BrushEngine::new();
    engine.inject_state(&initial_state());
    let result = engine.execute("echo hello");
    assert_eq!(result.last_exit_code, 0);
}

#[test]
fn test_brush_env_capture() {
    let mut engine = BrushEngine::new();
    engine.inject_state(&initial_state());
    let result = engine.execute("export FOO=test_value_brush");
    assert_eq!(result.env.get("FOO").unwrap(), "test_value_brush");
}

#[test]
fn test_brush_cwd_capture() {
    let mut engine = BrushEngine::new();
    engine.inject_state(&initial_state());
    let result = engine.execute("cd /tmp");
    assert!(
        result.cwd == PathBuf::from("/tmp") || result.cwd == PathBuf::from("/private/tmp"),
        "unexpected cwd: {:?}",
        result.cwd
    );
}

#[test]
fn test_brush_state_persistence() {
    let mut engine = BrushEngine::new();
    engine.inject_state(&initial_state());
    let _state1 = engine.execute("export PERSIST_BRUSH=hello");
    // Engine persists state across commands (no re-inject needed)
    let state2 = engine.execute("echo $PERSIST_BRUSH");
    assert_eq!(state2.last_exit_code, 0);
    assert_eq!(state2.env.get("PERSIST_BRUSH").unwrap(), "hello");
}

// --- Cross-engine tests ---

#[test]
fn test_env_brush_to_nushell() {
    let mut brush = BrushEngine::new();
    brush.inject_state(&initial_state());
    let brush_state = brush.execute("export CROSS=hello_from_brush");
    assert_eq!(brush_state.env.get("CROSS").unwrap(), "hello_from_brush");

    // Inject brush state into nushell engine
    let mut nu = NushellEngine::new(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)));
    nu.inject_state(&brush_state);
    let nu_state = nu.execute("echo $env.CROSS");
    assert_eq!(nu_state.last_exit_code, 0);
    assert_eq!(nu_state.env.get("CROSS").unwrap(), "hello_from_brush");
}

#[test]
fn test_cwd_brush_to_nushell() {
    let mut brush = BrushEngine::new();
    brush.inject_state(&initial_state());
    let brush_state = brush.execute("cd /tmp");

    let mut nu = NushellEngine::new(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)));
    nu.inject_state(&brush_state);
    let nu_state = nu.execute("pwd");
    assert!(
        nu_state.cwd == PathBuf::from("/tmp") || nu_state.cwd == PathBuf::from("/private/tmp"),
        "unexpected cwd: {:?}",
        nu_state.cwd
    );
}

// SIGINT handling is tested manually (not integration tests).
