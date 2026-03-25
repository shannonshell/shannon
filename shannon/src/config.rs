use serde::Deserialize;

use crate::shell::config_dir;

/// Top-level shannon configuration, loaded from config.toml.
#[derive(Deserialize, Default)]
pub struct ShannonConfig {
    /// Ordered list of shells for Shift+Tab rotation. First is default.
    pub toggle: Option<Vec<String>>,
    /// Deprecated: use `toggle` instead. Kept for backward compat.
    pub default_shell: Option<String>,
    /// AI mode configuration.
    #[serde(default)]
    pub ai: AiConfig,
    /// Theme configuration.
    #[serde(default)]
    pub theme: ThemeConfig,
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

/// Theme configuration.
#[derive(Deserialize, Default)]
pub struct ThemeConfig {
    pub name: Option<String>,
    pub keyword: Option<String>,
    pub command: Option<String>,
    pub string: Option<String>,
    pub number: Option<String>,
    pub variable: Option<String>,
    pub operator: Option<String>,
    pub comment: Option<String>,
    pub error: Option<String>,
    pub foreground: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub prompt: Option<String>,
    pub hint: Option<String>,
    pub ai_badge: Option<String>,
}

/// Built-in shell names and their highlighters.
const BUILTIN_SHELLS: &[(&str, &str)] = &[
    ("nu", "nushell"),
    ("brush", "bash"),
];

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

    /// Returns the ordered list of shell names for the Shift+Tab rotation.
    pub fn shell_order(&self) -> Vec<String> {
        let known: Vec<&str> = BUILTIN_SHELLS.iter().map(|(name, _)| *name).collect();

        if let Some(toggle) = &self.toggle {
            let mut result = Vec::new();
            for name in toggle {
                if known.contains(&name.as_str()) {
                    result.push(name.clone());
                } else {
                    eprintln!("shannon: unknown shell in toggle list: {name}");
                }
            }
            return result;
        }

        // Default order
        let mut result: Vec<String> = known.iter().map(|s| s.to_string()).collect();

        // Backward compat: default_shell moves that shell to front
        if let Some(default_name) = &self.default_shell {
            if let Some(pos) = result.iter().position(|n| n == default_name) {
                let shell = result.remove(pos);
                result.insert(0, shell);
            }
        }

        result
    }

    /// Get the highlighter name for a built-in shell.
    pub fn highlighter_for(name: &str) -> Option<String> {
        BUILTIN_SHELLS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, h)| h.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_config() {
        let config = ShannonConfig::default();
        let shells = config.shell_order();
        assert_eq!(shells.len(), 2);
        assert_eq!(shells[0], "nu");
        assert_eq!(shells[1], "brush");
    }

    #[test]
    fn test_toggle_list() {
        let config: ShannonConfig =
            toml::from_str(r#"toggle = ["brush", "nu"]"#).unwrap();
        let shells = config.shell_order();
        assert_eq!(shells.len(), 2);
        assert_eq!(shells[0], "brush");
        assert_eq!(shells[1], "nu");
    }

    #[test]
    fn test_toggle_unknown_shell() {
        let config: ShannonConfig =
            toml::from_str(r#"toggle = ["nu", "nonexistent", "brush"]"#).unwrap();
        let shells = config.shell_order();
        assert_eq!(shells.len(), 2);
        assert_eq!(shells[0], "nu");
        assert_eq!(shells[1], "brush");
    }

    #[test]
    fn test_default_shell_backward_compat() {
        let config: ShannonConfig =
            toml::from_str(r#"default_shell = "brush""#).unwrap();
        let shells = config.shell_order();
        assert_eq!(shells[0], "brush");
        assert_eq!(shells.len(), 2);
    }

    #[test]
    fn test_highlighter_for() {
        assert_eq!(
            ShannonConfig::highlighter_for("nu"),
            Some("nushell".to_string())
        );
        assert_eq!(
            ShannonConfig::highlighter_for("brush"),
            Some("bash".to_string())
        );
        assert_eq!(ShannonConfig::highlighter_for("unknown"), None);
    }
}
