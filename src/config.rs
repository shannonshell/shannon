use std::collections::HashMap;

use serde::Deserialize;

use crate::shell::config_dir;

/// Top-level shannon configuration, loaded from config.toml.
#[derive(Deserialize, Default)]
pub struct ShannonConfig {
    pub default_shell: Option<String>,
    #[serde(default)]
    pub shells: HashMap<String, ShellConfig>,
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
let __shannon_out = (try { {{command}} } catch { |e| $e.rendered | print -e; null })
if ($__shannon_out != null) and (($__shannon_out | describe) != "nothing") { $__shannon_out | print }
let shannon_exit = (if ($env | get -o LAST_EXIT_CODE | is-not-empty) { $env.LAST_EXIT_CODE } else { 0 })
$env | reject config? | insert __SHANNON_CWD (pwd) | insert __SHANNON_EXIT ($shannon_exit | into string) | to json --serialize | save --force '{{temp_path}}'"#;

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
    ]
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

    /// Returns the ordered list of shells: built-in defaults merged with user config.
    /// The default_shell (if set) is moved to the front.
    /// Also respects SHANNON_DEFAULT_SHELL env var as fallback.
    pub fn shells(&self, env_default: Option<&str>) -> Vec<(String, ShellConfig)> {
        let mut shells: Vec<(String, ShellConfig)> = Vec::new();

        // Start with built-in defaults
        for (name, config) in builtin_shells() {
            if let Some(user_config) = self.shells.get(&name) {
                // User overrides a built-in
                shells.push((name, user_config.clone()));
            } else {
                shells.push((name, config));
            }
        }

        // Add user-defined shells that aren't built-in
        for (name, config) in &self.shells {
            if !shells.iter().any(|(n, _)| n == name) {
                shells.push((name.clone(), config.clone()));
            }
        }

        // Determine default shell: config.toml > env var > first in list
        let default = self
            .default_shell
            .as_deref()
            .or(env_default);

        if let Some(default_name) = default {
            if let Some(pos) = shells.iter().position(|(n, _)| n == default_name) {
                let shell = shells.remove(pos);
                shells.insert(0, shell);
            }
        }

        shells
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
        let shells = config.shells(None);
        assert_eq!(shells.len(), 3);
        assert_eq!(shells[0].0, "bash");
        assert_eq!(shells[1].0, "nu");
        assert_eq!(shells[2].0, "fish");
    }

    #[test]
    fn test_default_shell() {
        let config = ShannonConfig {
            default_shell: Some("nu".to_string()),
            shells: HashMap::new(),
        };
        let shells = config.shells(None);
        assert_eq!(shells[0].0, "nu");
    }

    #[test]
    fn test_env_default_fallback() {
        let config = ShannonConfig::default();
        let shells = config.shells(Some("fish"));
        assert_eq!(shells[0].0, "fish");
    }

    #[test]
    fn test_config_overrides_env() {
        let config = ShannonConfig {
            default_shell: Some("nu".to_string()),
            shells: HashMap::new(),
        };
        // config.toml default takes precedence over env var
        let shells = config.shells(Some("fish"));
        assert_eq!(shells[0].0, "nu");
    }

    #[test]
    fn test_custom_shell() {
        let mut shells_map = HashMap::new();
        shells_map.insert(
            "zsh".to_string(),
            ShellConfig {
                binary: "zsh".to_string(),
                wrapper: "{{command}}".to_string(),
                parser: "env".to_string(),
                highlighter: Some("bash".to_string()),
                init: None,
            },
        );
        let config = ShannonConfig {
            default_shell: None,
            shells: shells_map,
        };
        let shells = config.shells(None);
        assert_eq!(shells.len(), 4); // 3 built-in + zsh
        assert!(shells.iter().any(|(n, _)| n == "zsh"));
    }

    #[test]
    fn test_override_builtin() {
        let mut shells_map = HashMap::new();
        shells_map.insert(
            "bash".to_string(),
            ShellConfig {
                binary: "/custom/bash".to_string(),
                wrapper: "custom {{command}}".to_string(),
                parser: "bash".to_string(),
                highlighter: Some("bash".to_string()),
                init: None,
            },
        );
        let config = ShannonConfig {
            default_shell: None,
            shells: shells_map,
        };
        let shells = config.shells(None);
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
    fn test_toml_parse() {
        let toml_str = r#"
default_shell = "nu"

[shells.zsh]
binary = "zsh"
wrapper = "{{command}}"
parser = "env"
highlighter = "bash"
"#;
        let config: ShannonConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.default_shell.as_deref(), Some("nu"));
        assert!(config.shells.contains_key("zsh"));
        assert_eq!(config.shells["zsh"].binary, "zsh");
    }
}
