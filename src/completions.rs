use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

static TABLE: LazyLock<CompletionTable> = LazyLock::new(|| {
    let json = include_str!(concat!(env!("OUT_DIR"), "/completions.json"));
    let specs: HashMap<String, CommandSpec> = serde_json::from_str(json).unwrap_or_default();
    CompletionTable { specs }
});

pub struct CompletionTable {
    specs: HashMap<String, CommandSpec>,
}

#[derive(Deserialize, Default)]
pub struct CommandSpec {
    pub subcommands: Vec<CompletionEntry>,
    pub global_flags: Vec<FlagEntry>,
    pub subcommand_flags: HashMap<String, Vec<FlagEntry>>,
}

#[derive(Deserialize)]
pub struct CompletionEntry {
    pub name: String,
    pub description: String,
}

#[derive(Deserialize)]
pub struct FlagEntry {
    pub short: Option<char>,
    pub long: Option<String>,
    pub description: String,
    pub takes_arg: bool,
}

impl CompletionTable {
    pub fn global() -> &'static CompletionTable {
        &TABLE
    }

    pub fn get(&self, command: &str) -> Option<&CommandSpec> {
        self.specs.get(command)
    }

    /// Given a command and the tokens after it, return completions for the
    /// current word being typed.
    pub fn complete(&self, command: &str, args: &[&str], prefix: &str) -> Vec<(String, String)> {
        let spec = match self.specs.get(command) {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Find the active subcommand (first non-flag arg)
        let active_sub = args
            .iter()
            .filter(|a| !a.starts_with('-'))
            .find(|a| spec.subcommands.iter().any(|s| s.name == ***a))
            .map(|s| s.to_string());

        if let Some(ref sub) = active_sub {
            // We have an active subcommand — complete its flags + global flags
            let mut results = Vec::new();
            let sub_flags = spec.subcommand_flags.get(sub.as_str());

            if prefix.starts_with("--") {
                // Long flag completion
                let pfx = &prefix[2..];
                if let Some(flags) = sub_flags {
                    for f in flags {
                        if let Some(ref l) = f.long {
                            if l.starts_with(pfx) {
                                results.push((format!("--{l}"), f.description.clone()));
                            }
                        }
                    }
                }
                for f in &spec.global_flags {
                    if let Some(ref l) = f.long {
                        if l.starts_with(pfx) {
                            results.push((format!("--{l}"), f.description.clone()));
                        }
                    }
                }
            } else if prefix.starts_with('-') && prefix.len() <= 2 {
                // Short flag completion
                let pfx = prefix.strip_prefix('-').unwrap_or("");
                if let Some(flags) = sub_flags {
                    for f in flags {
                        if let Some(s) = f.short {
                            let s_str = s.to_string();
                            if s_str.starts_with(pfx) {
                                results.push((format!("-{s}"), f.description.clone()));
                            }
                        }
                    }
                }
                for f in &spec.global_flags {
                    if let Some(s) = f.short {
                        let s_str = s.to_string();
                        if s_str.starts_with(pfx) {
                            results.push((format!("-{s}"), f.description.clone()));
                        }
                    }
                }
            }

            results
        } else {
            // No subcommand yet — complete subcommands
            let mut results = Vec::new();
            for sub in &spec.subcommands {
                if sub.name.starts_with(prefix) {
                    results.push((sub.name.clone(), sub.description.clone()));
                }
            }

            // Also include global flags if prefix starts with -
            if prefix.starts_with("--") {
                let pfx = &prefix[2..];
                for f in &spec.global_flags {
                    if let Some(ref l) = f.long {
                        if l.starts_with(pfx) {
                            results.push((format!("--{l}"), f.description.clone()));
                        }
                    }
                }
            } else if prefix.starts_with('-') && prefix.len() <= 2 {
                let pfx = prefix.strip_prefix('-').unwrap_or("");
                for f in &spec.global_flags {
                    if let Some(s) = f.short {
                        let s_str = s.to_string();
                        if s_str.starts_with(pfx) {
                            results.push((format!("-{s}"), f.description.clone()));
                        }
                    }
                }
            }

            results
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_table() {
        let table = CompletionTable::global();
        assert!(
            table.specs.len() > 100,
            "expected >100 commands, got {}",
            table.specs.len()
        );
    }

    #[test]
    fn test_git_subcommands() {
        let table = CompletionTable::global();
        let spec = table.get("git").expect("git should be in table");
        let names: Vec<&str> = spec.subcommands.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"commit"), "git should have commit subcommand");
        assert!(names.contains(&"push"), "git should have push subcommand");
        assert!(names.contains(&"pull"), "git should have pull subcommand");
    }

    #[test]
    fn test_git_commit_flags() {
        let table = CompletionTable::global();
        let spec = table.get("git").expect("git should be in table");
        let flags = spec.subcommand_flags.get("commit");
        assert!(flags.is_some(), "git commit should have flags");
        let flags = flags.unwrap();
        let longs: Vec<&str> = flags.iter().filter_map(|f| f.long.as_deref()).collect();
        assert!(
            longs.contains(&"message"),
            "git commit should have --message, got: {longs:?}"
        );
    }

    #[test]
    fn test_unknown_command() {
        let table = CompletionTable::global();
        let results = table.complete("zzz_nonexistent_cmd", &[], "");
        assert!(results.is_empty());
    }

    #[test]
    fn test_prefix_filter() {
        let table = CompletionTable::global();
        let results = table.complete("git", &[], "com");
        let names: Vec<&str> = results.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"commit"), "filtering by 'com' should include commit");
        assert!(
            !names.contains(&"push"),
            "filtering by 'com' should not include push"
        );
    }
}
