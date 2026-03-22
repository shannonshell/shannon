use std::collections::HashMap;

use serde::Deserialize;

use crate::shell::config_dir;

/// Top-level shannon configuration, loaded from config.toml.
#[derive(Deserialize, Default)]
pub struct ShannonConfig {
    /// Ordered list of shells for Shift+Tab rotation. First is default.
    pub toggle: Option<Vec<String>>,
    /// Deprecated: use `toggle` instead. Kept for backward compat.
    pub default_shell: Option<String>,
    #[serde(default)]
    pub shells: HashMap<String, ShellConfig>,
    /// AI mode configuration.
    #[serde(default)]
    pub ai: AiConfig,
}

/// Configuration for AI mode.
#[derive(Deserialize, Default, Clone)]
pub struct AiConfig {
    /// LLM provider (default: "anthropic")
    pub provider: Option<String>,
    /// Model name (default: "claude-sonnet-4-20250514")
    pub model: Option<String>,
    /// Environment variable name for the API key (default: "ANTHROPIC_API_KEY")
    pub api_key_env: Option<String>,
}

/// Configuration for a single shell.
#[derive(Deserialize, Clone)]
pub struct ShellConfig {
    pub binary: String,
    pub wrapper: String,
    #[serde(default = "default_parser")]
    pub parser: String,
    pub highlighter: Option<String>,
    pub init: Option<String>,
}

fn default_parser() -> String {
    "env".to_string()
}

// --- Built-in defaults ---

const BASH_WRAPPER: &str = r#"{{init}}
{{command}}
__shannon_ec=$?
(export -p; echo "__SHANNON_CWD=$(pwd)"; echo "__SHANNON_EXIT=$__shannon_ec") > '{{temp_path}}'
exit $__shannon_ec"#;

const NUSHELL_WRAPPER: &str = r#"{{init}}
{{command}}
let shannon_exit = (if ($env | get -o LAST_EXIT_CODE | is-not-empty) { $env.LAST_EXIT_CODE } else { 0 })
$env | reject config? | insert __SHANNON_CWD (pwd) | insert __SHANNON_EXIT ($shannon_exit | into string) | to json --serialize | save --force '{{temp_path}}'"#;

const ENV_WRAPPER: &str = r#"{{init}}
{{command}}
__shannon_ec=$?
env > '{{temp_path}}'
echo "__SHANNON_CWD=$(pwd)" >> '{{temp_path}}'
echo "__SHANNON_EXIT=$__shannon_ec" >> '{{temp_path}}'
exit $__shannon_ec"#;

const FISH_WRAPPER: &str = r#"{{init}}
{{command}}
set __shannon_ec $status
env > '{{temp_path}}'
echo "__SHANNON_CWD="(pwd) >> '{{temp_path}}'
echo "__SHANNON_EXIT=$__shannon_ec" >> '{{temp_path}}'
exit $__shannon_ec"#;

fn builtin_shells() -> Vec<(String, ShellConfig)> {
    vec![
        (
            "bash".to_string(),
            ShellConfig {
                binary: "bash".to_string(),
                wrapper: BASH_WRAPPER.to_string(),
                parser: "bash".to_string(),
                highlighter: Some("bash".to_string()),
                init: None,
            },
        ),
        (
            "nu".to_string(),
            ShellConfig {
                binary: "nu".to_string(),
                wrapper: NUSHELL_WRAPPER.to_string(),
                parser: "nushell".to_string(),
                highlighter: Some("nushell".to_string()),
                init: None,
            },
        ),
        (
            "fish".to_string(),
            ShellConfig {
                binary: "fish".to_string(),
                wrapper: FISH_WRAPPER.to_string(),
                parser: "env".to_string(),
                highlighter: Some("fish".to_string()),
                init: None,
            },
        ),
        (
            "zsh".to_string(),
            ShellConfig {
                binary: "zsh".to_string(),
                wrapper: ENV_WRAPPER.to_string(),
                parser: "env".to_string(),
                highlighter: Some("bash".to_string()),
                init: None,
            },
        ),
    ]
}

/// Build the full map of available shells (built-in + custom).
fn all_shells(config: &ShannonConfig) -> HashMap<String, ShellConfig> {
    let mut map = HashMap::new();

    // Built-in defaults
    for (name, shell_config) in builtin_shells() {
        map.insert(name, shell_config);
    }

    // User overrides and custom shells
    for (name, shell_config) in &config.shells {
        map.insert(name.clone(), shell_config.clone());
    }

    map
}

impl ShannonConfig {
    /// Load config from config.toml. Returns defaults if file doesn't exist.
    pub fn load() -> Self {
        let config_path = config_dir().join("config.toml");
        if !config_path.exists() {
            return ShannonConfig::default();
        }

        let contents = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("shannon: failed to read config.toml: {e}");
                std::process::exit(1);
            }
        };

        match toml::from_str(&contents) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("shannon: invalid config.toml: {e}");
                std::process::exit(1);
            }
        }
    }

    /// Returns the ordered list of shells for the Shift+Tab rotation.
    ///
    /// If `toggle` is set, returns those shells in order (duplicates allowed).
    /// If only `default_shell` is set (backward compat), puts that shell first.
    /// If neither is set, returns all built-in + custom shells in default order.
    pub fn shells(&self) -> Vec<(String, ShellConfig)> {
        let available = all_shells(self);

        if let Some(toggle) = &self.toggle {
            // Toggle list: return shells in the specified order
            let mut result = Vec::new();
            for name in toggle {
                if let Some(config) = available.get(name) {
                    result.push((name.clone(), config.clone()));
                } else {
                    eprintln!("shannon: unknown shell in toggle list: {name}");
                }
            }
            return result;
        }

        // No toggle list — return all shells in default order
        let mut result = Vec::new();
        // Built-in shells first, in their defined order
        for (name, _) in builtin_shells() {
            if let Some(config) = available.get(&name) {
                result.push((name, config.clone()));
            }
        }
        // Custom shells after
        for (name, config) in &self.shells {
            if !result.iter().any(|(n, _)| n == name) {
                result.push((name.clone(), config.clone()));
            }
        }

        // Backward compat: default_shell moves that shell to front
        if let Some(default_name) = &self.default_shell {
            if let Some(pos) = result.iter().position(|(n, _)| n == default_name) {
                let shell = result.remove(pos);
                result.insert(0, shell);
            }
        }

        result
    }
}

/// Expand a wrapper template by replacing placeholders.
pub fn expand_wrapper(
    wrapper: &str,
    command: &str,
    temp_path: &str,
    init_content: &str,
) -> String {
    wrapper
        .replace("{{command}}", command)
        .replace("{{temp_path}}", temp_path)
        .replace("{{init}}", init_content)
}

/// Read the init file for a shell, returning its contents or empty string.
pub fn read_init_file(init_path: Option<&str>) -> String {
    let path = match init_path {
        Some(p) => config_dir().join(p),
        None => return String::new(),
    };
    std::fs::read_to_string(&path).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_config() {
        let config = ShannonConfig::default();
        let shells = config.shells();
        assert_eq!(shells.len(), 4);
        assert_eq!(shells[0].0, "bash");
        assert_eq!(shells[1].0, "nu");
        assert_eq!(shells[2].0, "fish");
        assert_eq!(shells[3].0, "zsh");
    }

    #[test]
    fn test_toggle_list() {
        let config: ShannonConfig = toml::from_str(r#"toggle = ["nu", "bash"]"#).unwrap();
        let shells = config.shells();
        assert_eq!(shells.len(), 2);
        assert_eq!(shells[0].0, "nu");
        assert_eq!(shells[1].0, "bash");
    }

    #[test]
    fn test_toggle_unknown_shell() {
        let config: ShannonConfig =
            toml::from_str(r#"toggle = ["nu", "nonexistent", "bash"]"#).unwrap();
        let shells = config.shells();
        assert_eq!(shells.len(), 2);
        assert_eq!(shells[0].0, "nu");
        assert_eq!(shells[1].0, "bash");
    }

    #[test]
    fn test_toggle_duplicates() {
        let config: ShannonConfig =
            toml::from_str(r#"toggle = ["fish", "bash", "fish"]"#).unwrap();
        let shells = config.shells();
        assert_eq!(shells.len(), 3);
        assert_eq!(shells[0].0, "fish");
        assert_eq!(shells[1].0, "bash");
        assert_eq!(shells[2].0, "fish");
    }

    #[test]
    fn test_toggle_with_custom_shell() {
        let toml_str = r#"
toggle = ["zsh", "nu"]

[shells.elvish]
binary = "elvish"
wrapper = "{{command}}"
"#;
        let config: ShannonConfig = toml::from_str(toml_str).unwrap();
        let shells = config.shells();
        // elvish is defined but not in toggle, so not returned
        assert_eq!(shells.len(), 2);
        assert_eq!(shells[0].0, "zsh");
        assert_eq!(shells[1].0, "nu");
    }

    #[test]
    fn test_default_shell_backward_compat() {
        let config: ShannonConfig =
            toml::from_str(r#"default_shell = "nu""#).unwrap();
        let shells = config.shells();
        assert_eq!(shells[0].0, "nu");
        assert_eq!(shells.len(), 4); // all built-ins, nu first
    }

    #[test]
    fn test_custom_shell_in_toggle() {
        let toml_str = r#"
toggle = ["elvish", "bash"]

[shells.elvish]
binary = "elvish"
wrapper = "{{command}}"
"#;
        let config: ShannonConfig = toml::from_str(toml_str).unwrap();
        let shells = config.shells();
        assert_eq!(shells.len(), 2);
        assert_eq!(shells[0].0, "elvish");
        assert_eq!(shells[0].1.binary, "elvish");
        assert_eq!(shells[1].0, "bash");
    }

    #[test]
    fn test_override_builtin() {
        let toml_str = r#"
[shells.bash]
binary = "/custom/bash"
wrapper = "custom {{command}}"
parser = "bash"
highlighter = "bash"
"#;
        let config: ShannonConfig = toml::from_str(toml_str).unwrap();
        let shells = config.shells();
        assert_eq!(shells[0].1.binary, "/custom/bash");
    }

    #[test]
    fn test_expand_wrapper() {
        let result = expand_wrapper(
            "{{init}}\n{{command}}\nenv > '{{temp_path}}'",
            "echo hello",
            "/tmp/test.env",
            "# init",
        );
        assert!(result.contains("echo hello"));
        assert!(result.contains("/tmp/test.env"));
        assert!(result.contains("# init"));
    }

    #[test]
    fn test_expand_wrapper_empty_init() {
        let result = expand_wrapper("{{init}}{{command}}", "ls", "/tmp/t", "");
        assert_eq!(result, "ls");
    }

    #[test]
    fn test_toml_parse_toggle() {
        let toml_str = r#"
toggle = ["nu", "fish"]

[shells.zsh]
binary = "zsh"
wrapper = "{{command}}"
parser = "env"
highlighter = "bash"
"#;
        let config: ShannonConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.toggle.as_deref(), Some(&["nu".to_string(), "fish".to_string()][..]));
        assert!(config.shells.contains_key("zsh"));
    }
}
