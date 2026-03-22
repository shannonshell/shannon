use std::path::PathBuf;
use std::process::Command;

use shannon::executor::execute_command;
use shannon::shell::{ShellKind, ShellState};

fn has_shell(shell: ShellKind) -> bool {
    Command::new(shell.binary())
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

fn initial_state() -> ShellState {
    ShellState::from_current_env()
}

// --- Bash tests ---

#[test]
fn test_bash_echo() {
    assert!(has_shell(ShellKind::Bash), "bash not found");
    let state = initial_state();
    let result = execute_command(ShellKind::Bash, "echo hello", &state).unwrap();
    assert_eq!(result.last_exit_code, 0);
}

#[test]
fn test_bash_env_capture() {
    assert!(has_shell(ShellKind::Bash), "bash not found");
    let state = initial_state();
    let result =
        execute_command(ShellKind::Bash, "export FOO=test_value_123", &state).unwrap();
    assert_eq!(result.env.get("FOO").unwrap(), "test_value_123");
}

#[test]
fn test_bash_cwd_capture() {
    assert!(has_shell(ShellKind::Bash), "bash not found");
    let state = initial_state();
    let result = execute_command(ShellKind::Bash, "cd /tmp", &state).unwrap();
    // macOS symlinks /tmp -> /private/tmp
    assert!(
        result.cwd == PathBuf::from("/tmp") || result.cwd == PathBuf::from("/private/tmp"),
        "unexpected cwd: {:?}",
        result.cwd
    );
}

#[test]
fn test_bash_exit_code() {
    assert!(has_shell(ShellKind::Bash), "bash not found");
    let state = initial_state();
    let result = execute_command(ShellKind::Bash, "false", &state).unwrap();
    assert_ne!(result.last_exit_code, 0);
}

#[test]
fn test_bash_env_persistence() {
    assert!(has_shell(ShellKind::Bash), "bash not found");
    let state = initial_state();

    // First command: set env var
    let state2 = execute_command(ShellKind::Bash, "export PERSIST_TEST=hello", &state).unwrap();
    assert_eq!(state2.env.get("PERSIST_TEST").unwrap(), "hello");

    // Second command: use the env var — it should be available
    let state3 = execute_command(ShellKind::Bash, "echo $PERSIST_TEST", &state2).unwrap();
    assert_eq!(state3.last_exit_code, 0);
    // The env var should still be present
    assert_eq!(state3.env.get("PERSIST_TEST").unwrap(), "hello");
}

// --- Nushell tests ---

#[test]

fn test_nushell_echo() {
    assert!(has_shell(ShellKind::Nushell), "nu not found");
    let state = initial_state();
    let result = execute_command(ShellKind::Nushell, "print hello", &state).unwrap();
    assert_eq!(result.last_exit_code, 0);
}

#[test]

fn test_nushell_env_capture() {
    assert!(has_shell(ShellKind::Nushell), "nu not found");
    let state = initial_state();
    let result = execute_command(
        ShellKind::Nushell,
        r#"$env.FOO = "test_value_456""#,
        &state,
    )
    .unwrap();
    assert_eq!(result.env.get("FOO").unwrap(), "test_value_456");
}

#[test]

fn test_nushell_cwd_capture() {
    assert!(has_shell(ShellKind::Nushell), "nu not found");
    let state = initial_state();
    let result = execute_command(ShellKind::Nushell, "cd /tmp", &state).unwrap();
    assert!(
        result.cwd == PathBuf::from("/tmp") || result.cwd == PathBuf::from("/private/tmp"),
        "unexpected cwd: {:?}",
        result.cwd
    );
}

#[test]

fn test_nushell_exit_code() {
    assert!(has_shell(ShellKind::Nushell), "nu not found");
    let state = initial_state();
    let result = execute_command(ShellKind::Nushell, "exit 1", &state).unwrap();
    assert_ne!(result.last_exit_code, 0);
}

// --- Fish tests ---

#[test]
fn test_fish_echo() {
    if !has_shell(ShellKind::Fish) {
        return;
    }
    let state = initial_state();
    let result = execute_command(ShellKind::Fish, "echo hello", &state).unwrap();
    assert_eq!(result.last_exit_code, 0);
}

#[test]
fn test_fish_env_capture() {
    if !has_shell(ShellKind::Fish) {
        return;
    }
    let state = initial_state();
    let result =
        execute_command(ShellKind::Fish, "set -gx FOO test_value_789", &state).unwrap();
    assert_eq!(result.env.get("FOO").unwrap(), "test_value_789");
}

#[test]
fn test_fish_cwd_capture() {
    if !has_shell(ShellKind::Fish) {
        return;
    }
    let state = initial_state();
    let result = execute_command(ShellKind::Fish, "cd /tmp", &state).unwrap();
    assert!(
        result.cwd == PathBuf::from("/tmp") || result.cwd == PathBuf::from("/private/tmp"),
        "unexpected cwd: {:?}",
        result.cwd
    );
}

#[test]
fn test_fish_exit_code() {
    if !has_shell(ShellKind::Fish) {
        return;
    }
    let state = initial_state();
    let result = execute_command(ShellKind::Fish, "false", &state).unwrap();
    assert_ne!(result.last_exit_code, 0);
}

// --- Cross-shell tests ---

#[test]

fn test_env_bash_to_nushell() {
    assert!(has_shell(ShellKind::Bash), "bash not found");
    assert!(has_shell(ShellKind::Nushell), "nu not found");

    let state = initial_state();

    // Set env in bash
    let bash_state =
        execute_command(ShellKind::Bash, "export CROSS=hello_from_bash", &state).unwrap();
    assert_eq!(bash_state.env.get("CROSS").unwrap(), "hello_from_bash");

    // Execute in nushell with bash's state — env should carry over
    let nu_state =
        execute_command(ShellKind::Nushell, "print $env.CROSS", &bash_state).unwrap();
    assert_eq!(nu_state.last_exit_code, 0);
    assert_eq!(nu_state.env.get("CROSS").unwrap(), "hello_from_bash");
}

#[test]

fn test_cwd_bash_to_nushell() {
    assert!(has_shell(ShellKind::Bash), "bash not found");
    assert!(has_shell(ShellKind::Nushell), "nu not found");

    let state = initial_state();

    // cd in bash
    let bash_state = execute_command(ShellKind::Bash, "cd /tmp", &state).unwrap();

    // Execute in nushell with bash's state — cwd should carry over
    let nu_state = execute_command(ShellKind::Nushell, "print (pwd)", &bash_state).unwrap();
    assert!(
        nu_state.cwd == PathBuf::from("/tmp") || nu_state.cwd == PathBuf::from("/private/tmp"),
        "unexpected cwd: {:?}",
        nu_state.cwd
    );
}

#[test]
fn test_env_bash_to_fish() {
    if !has_shell(ShellKind::Bash) || !has_shell(ShellKind::Fish) {
        return;
    }

    let state = initial_state();

    // Set env in bash
    let bash_state =
        execute_command(ShellKind::Bash, "export CROSS_FISH=hello_from_bash", &state).unwrap();
    assert_eq!(
        bash_state.env.get("CROSS_FISH").unwrap(),
        "hello_from_bash"
    );

    // Execute in fish with bash's state — env should carry over
    let fish_state =
        execute_command(ShellKind::Fish, "echo $CROSS_FISH", &bash_state).unwrap();
    assert_eq!(fish_state.last_exit_code, 0);
    assert_eq!(
        fish_state.env.get("CROSS_FISH").unwrap(),
        "hello_from_bash"
    );
}
