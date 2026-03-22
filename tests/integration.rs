use std::path::PathBuf;
use std::process::Command;

use shannon::config::ShellConfig;
use shannon::executor::execute_command;
use shannon::shell::ShellState;

fn has_binary(binary: &str) -> bool {
    Command::new(binary)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

fn initial_state() -> ShellState {
    ShellState::from_current_env()
}

fn bash_config() -> ShellConfig {
    shannon::config::ShannonConfig::default()
        .shells()
        .into_iter()
        .find(|(name, _)| name == "bash")
        .unwrap()
        .1
}

fn nushell_config() -> ShellConfig {
    shannon::config::ShannonConfig::default()
        .shells()
        .into_iter()
        .find(|(name, _)| name == "nu")
        .unwrap()
        .1
}

fn fish_config() -> ShellConfig {
    shannon::config::ShannonConfig::default()
        .shells()
        .into_iter()
        .find(|(name, _)| name == "fish")
        .unwrap()
        .1
}

// --- Bash tests ---

#[test]
fn test_bash_echo() {
    assert!(has_binary("bash"), "bash not found");
    let state = initial_state();
    let result = execute_command(&bash_config(), "echo hello", &state).unwrap();
    assert_eq!(result.last_exit_code, 0);
}

#[test]
fn test_bash_env_capture() {
    assert!(has_binary("bash"), "bash not found");
    let state = initial_state();
    let result = execute_command(&bash_config(), "export FOO=test_value_123", &state).unwrap();
    assert_eq!(result.env.get("FOO").unwrap(), "test_value_123");
}

#[test]
fn test_bash_cwd_capture() {
    assert!(has_binary("bash"), "bash not found");
    let state = initial_state();
    let result = execute_command(&bash_config(), "cd /tmp", &state).unwrap();
    assert!(
        result.cwd == PathBuf::from("/tmp") || result.cwd == PathBuf::from("/private/tmp"),
        "unexpected cwd: {:?}",
        result.cwd
    );
}

#[test]
fn test_bash_exit_code() {
    assert!(has_binary("bash"), "bash not found");
    let state = initial_state();
    let result = execute_command(&bash_config(), "false", &state).unwrap();
    assert_ne!(result.last_exit_code, 0);
}

#[test]
fn test_bash_env_persistence() {
    assert!(has_binary("bash"), "bash not found");
    let state = initial_state();

    let state2 = execute_command(&bash_config(), "export PERSIST_TEST=hello", &state).unwrap();
    assert_eq!(state2.env.get("PERSIST_TEST").unwrap(), "hello");

    let state3 = execute_command(&bash_config(), "echo $PERSIST_TEST", &state2).unwrap();
    assert_eq!(state3.last_exit_code, 0);
    assert_eq!(state3.env.get("PERSIST_TEST").unwrap(), "hello");
}

// --- Nushell tests ---

#[test]
fn test_nushell_echo() {
    if !has_binary("nu") {
        return;
    }
    let state = initial_state();
    let result = execute_command(&nushell_config(), "print hello", &state).unwrap();
    assert_eq!(result.last_exit_code, 0);
}

#[test]
fn test_nushell_env_capture() {
    if !has_binary("nu") {
        return;
    }
    let state = initial_state();
    let result = execute_command(
        &nushell_config(),
        r#"$env.FOO = "test_value_456""#,
        &state,
    )
    .unwrap();
    assert_eq!(result.env.get("FOO").unwrap(), "test_value_456");
}

#[test]
fn test_nushell_cwd_capture() {
    if !has_binary("nu") {
        return;
    }
    let state = initial_state();
    let result = execute_command(&nushell_config(), "cd /tmp", &state).unwrap();
    assert!(
        result.cwd == PathBuf::from("/tmp") || result.cwd == PathBuf::from("/private/tmp"),
        "unexpected cwd: {:?}",
        result.cwd
    );
}

#[test]
fn test_nushell_exit_code() {
    if !has_binary("nu") {
        return;
    }
    let state = initial_state();
    let result = execute_command(&nushell_config(), "exit 1", &state).unwrap();
    assert_ne!(result.last_exit_code, 0);
}

// --- Fish tests ---

#[test]
fn test_fish_echo() {
    if !has_binary("fish") {
        return;
    }
    let state = initial_state();
    let result = execute_command(&fish_config(), "echo hello", &state).unwrap();
    assert_eq!(result.last_exit_code, 0);
}

#[test]
fn test_fish_env_capture() {
    if !has_binary("fish") {
        return;
    }
    let state = initial_state();
    let result =
        execute_command(&fish_config(), "set -gx FOO test_value_789", &state).unwrap();
    assert_eq!(result.env.get("FOO").unwrap(), "test_value_789");
}

#[test]
fn test_fish_cwd_capture() {
    if !has_binary("fish") {
        return;
    }
    let state = initial_state();
    let result = execute_command(&fish_config(), "cd /tmp", &state).unwrap();
    assert!(
        result.cwd == PathBuf::from("/tmp") || result.cwd == PathBuf::from("/private/tmp"),
        "unexpected cwd: {:?}",
        result.cwd
    );
}

#[test]
fn test_fish_exit_code() {
    if !has_binary("fish") {
        return;
    }
    let state = initial_state();
    let result = execute_command(&fish_config(), "false", &state).unwrap();
    assert_ne!(result.last_exit_code, 0);
}

// --- Cross-shell tests ---

#[test]
fn test_env_bash_to_nushell() {
    if !has_binary("bash") || !has_binary("nu") {
        return;
    }

    let state = initial_state();

    let bash_state =
        execute_command(&bash_config(), "export CROSS=hello_from_bash", &state).unwrap();
    assert_eq!(bash_state.env.get("CROSS").unwrap(), "hello_from_bash");

    let nu_state =
        execute_command(&nushell_config(), "print $env.CROSS", &bash_state).unwrap();
    assert_eq!(nu_state.last_exit_code, 0);
    assert_eq!(nu_state.env.get("CROSS").unwrap(), "hello_from_bash");
}

#[test]
fn test_cwd_bash_to_nushell() {
    if !has_binary("bash") || !has_binary("nu") {
        return;
    }

    let state = initial_state();

    let bash_state = execute_command(&bash_config(), "cd /tmp", &state).unwrap();

    let nu_state = execute_command(&nushell_config(), "print (pwd)", &bash_state).unwrap();
    assert!(
        nu_state.cwd == PathBuf::from("/tmp") || nu_state.cwd == PathBuf::from("/private/tmp"),
        "unexpected cwd: {:?}",
        nu_state.cwd
    );
}

#[test]
fn test_env_bash_to_fish() {
    if !has_binary("bash") || !has_binary("fish") {
        return;
    }

    let state = initial_state();

    let bash_state =
        execute_command(&bash_config(), "export CROSS_FISH=hello_from_bash", &state).unwrap();
    assert_eq!(
        bash_state.env.get("CROSS_FISH").unwrap(),
        "hello_from_bash"
    );

    let fish_state =
        execute_command(&fish_config(), "echo $CROSS_FISH", &bash_state).unwrap();
    assert_eq!(fish_state.last_exit_code, 0);
    assert_eq!(
        fish_state.env.get("CROSS_FISH").unwrap(),
        "hello_from_bash"
    );
}
