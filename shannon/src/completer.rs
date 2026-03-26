use std::path::{Path, PathBuf};

use reedline::{Completer, Span, Suggestion};

use crate::completions::CompletionTable;

pub struct ShannonCompleter;

impl ShannonCompleter {
    pub fn new() -> Self {
        Self
    }

    fn cwd() -> PathBuf {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    fn complete_file(&self, line: &str, pos: usize) -> Vec<Suggestion> {
        let cwd = Self::cwd();
        let line = &line[..pos];
        let word_start = line.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let word = &line[word_start..];

        if word.is_empty() {
            return Vec::new();
        }

        let home = dirs::home_dir();
        let (search_path, display_prefix) = if word == "~" {
            match &home {
                Some(h) => (h.clone(), "~/".to_string()),
                None => return Vec::new(),
            }
        } else if let Some(rest) = word.strip_prefix("~/") {
            match &home {
                Some(h) => {
                    if let Some((dir, _prefix)) = rest.rsplit_once('/') {
                        (h.join(dir), format!("~/{dir}/"))
                    } else {
                        (h.clone(), "~/".to_string())
                    }
                }
                None => return Vec::new(),
            }
        } else if let Some((dir, _prefix)) = word.rsplit_once('/') {
            let resolved = if Path::new(dir).is_absolute() {
                PathBuf::from(dir)
            } else {
                cwd.join(dir)
            };
            (resolved, format!("{dir}/"))
        } else {
            (cwd.clone(), String::new())
        };

        let filename_prefix = if word == "~" {
            ""
        } else if word.ends_with('/') {
            ""
        } else {
            word.rsplit_once('/').map(|(_, p)| p).unwrap_or(word)
        };

        let show_hidden = filename_prefix.starts_with('.');

        let entries = match std::fs::read_dir(&search_path) {
            Ok(entries) => entries,
            Err(_) => return Vec::new(),
        };

        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if !show_hidden && name_str.starts_with('.') {
                continue;
            }

            if !name_str.starts_with(filename_prefix) {
                continue;
            }

            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);

            let value = if is_dir {
                format!("{}{}/", display_prefix, name_str)
            } else {
                format!("{}{}", display_prefix, name_str)
            };

            let suggestion = Suggestion {
                value,
                description: None,
                style: None,
                extra: None,
                span: Span::new(word_start, pos),
                append_whitespace: !is_dir,
                display_override: None,
                match_indices: None,
            };

            if is_dir {
                dirs.push(suggestion);
            } else {
                files.push(suggestion);
            }
        }

        dirs.sort_by(|a, b| a.value.cmp(&b.value));
        files.sort_by(|a, b| a.value.cmp(&b.value));
        dirs.extend(files);
        dirs
    }
}

impl Completer for ShannonCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let line_to_pos = &line[..pos];

        // Tokenize the line up to cursor
        let tokens: Vec<&str> = line_to_pos.split_whitespace().collect();

        // If we're on the first word or the line is empty, do file completion
        if tokens.is_empty() || (tokens.len() == 1 && !line_to_pos.ends_with(' ')) {
            return self.complete_file(line, pos);
        }

        let command = tokens[0];
        let table = CompletionTable::global();

        // Check if this command has completions
        if table.get(command).is_none() {
            return self.complete_file(line, pos);
        }

        // Determine the current word and args before it
        let word_start = line_to_pos.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let current_word = &line_to_pos[word_start..];
        let args: Vec<&str> = tokens[1..].to_vec();

        // If current word is being typed (not yet space-terminated), exclude it from args
        let args_before = if line_to_pos.ends_with(' ') {
            &args[..]
        } else if args.is_empty() {
            &[]
        } else {
            &args[..args.len() - 1]
        };

        let results = table.complete(command, args_before, current_word);

        if results.is_empty() {
            // Fall back to file completion
            return self.complete_file(line, pos);
        }

        results
            .into_iter()
            .map(|(value, desc)| Suggestion {
                value,
                description: if desc.is_empty() {
                    None
                } else {
                    Some(desc)
                },
                style: None,
                extra: None,
                span: Span::new(word_start, pos),
                append_whitespace: true,
                display_override: None,
                match_indices: None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "").unwrap();
        fs::write(root.join("src/lib.rs"), "").unwrap();
        fs::write(root.join("Cargo.toml"), "").unwrap();
        fs::write(root.join("Cargo.lock"), "").unwrap();
        fs::write(root.join(".gitignore"), "").unwrap();
        fs::create_dir(root.join(".hidden_dir")).unwrap();
        fs::write(root.join(".hidden_dir/secret.txt"), "").unwrap();
        fs::write(root.join("notes.txt"), "").unwrap();

        dir
    }

    // --- File completion tests (preserved from FileCompleter) ---

    #[test]
    fn test_complete_partial_filename() {
        let dir = setup_test_dir();
        let mut c = {
            std::env::set_current_dir(dir.path()).unwrap();
            ShannonCompleter::new()
        };
        let results = c.complete("cat Car", 7);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"Cargo.lock"), "expected Cargo.lock in {values:?}");
        assert!(values.contains(&"Cargo.toml"), "expected Cargo.toml in {values:?}");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_complete_directory_contents() {
        let dir = setup_test_dir();
        let mut c = {
            std::env::set_current_dir(dir.path()).unwrap();
            ShannonCompleter::new()
        };
        let results = c.complete("ls src/", 7);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"src/lib.rs"), "expected src/lib.rs in {values:?}");
        assert!(values.contains(&"src/main.rs"), "expected src/main.rs in {values:?}");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_complete_directory_trailing_slash() {
        let dir = setup_test_dir();
        let mut c = {
            std::env::set_current_dir(dir.path()).unwrap();
            ShannonCompleter::new()
        };
        let results = c.complete("cd sr", 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "src/");
        assert!(!results[0].append_whitespace);
    }

    #[test]
    fn test_complete_file_appends_space() {
        let dir = setup_test_dir();
        let mut c = {
            std::env::set_current_dir(dir.path()).unwrap();
            ShannonCompleter::new()
        };
        let results = c.complete("cat notes", 9);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "notes.txt");
        assert!(results[0].append_whitespace);
    }

    #[test]
    fn test_hidden_files_excluded() {
        let dir = setup_test_dir();
        let mut c = {
            std::env::set_current_dir(dir.path()).unwrap();
            ShannonCompleter::new()
        };
        let results = c.complete("ls C", 4);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        for v in &values {
            assert!(!v.starts_with('.'), "hidden file {v} should be excluded");
        }
    }

    #[test]
    fn test_hidden_files_included_with_dot_prefix() {
        let dir = setup_test_dir();
        let mut c = {
            std::env::set_current_dir(dir.path()).unwrap();
            ShannonCompleter::new()
        };
        let results = c.complete("ls .", 4);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&".gitignore"), "expected .gitignore in {values:?}");
        assert!(values.contains(&".hidden_dir/"), "expected .hidden_dir/ in {values:?}");
    }

    #[test]
    fn test_no_matches() {
        let dir = setup_test_dir();
        let mut c = {
            std::env::set_current_dir(dir.path()).unwrap();
            ShannonCompleter::new()
        };
        let results = c.complete("cat zzz", 7);
        assert!(results.is_empty());
    }

    #[test]
    fn test_sort_order() {
        let dir = setup_test_dir();
        let mut c = {
            std::env::set_current_dir(dir.path()).unwrap();
            ShannonCompleter::new()
        };
        // Use "myapp" (not in fish completions) to test pure file completion
        let results = c.complete("myapp Car", 9);
        assert_eq!(results[0].value, "Cargo.lock");
        assert_eq!(results[1].value, "Cargo.toml");

        let results = c.complete("myapp s", 7);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "src/");

        let root = dir.path();
        fs::write(root.join("aaa_file.txt"), "").unwrap();
        fs::create_dir(root.join("aaa_dir")).unwrap();
        let results = c.complete("myapp aaa", 9);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].value, "aaa_dir/");
        assert_eq!(results[1].value, "aaa_file.txt");
    }

    #[test]
    fn test_tilde_expansion() {
        let mut c = ShannonCompleter::new();
        let results = c.complete("ls ~/", 4);
        for s in &results {
            assert!(
                s.value.starts_with("~/"),
                "expected suggestion to start with ~/, got: {}",
                s.value
            );
        }
    }

    // --- Command completion tests ---

    #[test]
    fn test_command_completion_git() {
        let mut c = ShannonCompleter::new();
        let results = c.complete("git ", 4);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"commit"), "expected commit in git completions");
        assert!(values.contains(&"push"), "expected push in git completions");
    }

    #[test]
    fn test_command_completion_git_prefix() {
        let mut c = ShannonCompleter::new();
        let results = c.complete("git com", 7);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"commit"), "expected commit");
        assert!(!values.contains(&"push"), "push should be filtered out");
    }

    #[test]
    fn test_command_completion_git_commit_flags() {
        let mut c = ShannonCompleter::new();
        let results = c.complete("git commit --", 13);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(
            values.contains(&"--message"),
            "expected --message in git commit flags, got: {values:?}"
        );
    }
}
