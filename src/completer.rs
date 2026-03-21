use std::path::{Path, PathBuf};

use reedline::{Completer, Span, Suggestion};

pub struct FileCompleter {
    cwd: PathBuf,
}

impl FileCompleter {
    pub fn new() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    #[cfg(test)]
    fn with_cwd(cwd: PathBuf) -> Self {
        Self { cwd }
    }
}

impl Completer for FileCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let line = &line[..pos];

        // Extract the word being completed by scanning backward to whitespace
        let word_start = line.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let word = &line[word_start..];

        if word.is_empty() {
            return Vec::new();
        }

        // Handle tilde expansion
        let home = dirs::home_dir();
        let (search_path, display_prefix) = if word == "~" {
            // Just "~" — list home directory contents
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
                self.cwd.join(dir)
            };
            (resolved, format!("{dir}/"))
        } else {
            (self.cwd.clone(), String::new())
        };

        // Extract the filename prefix to match against
        let filename_prefix = if word == "~" {
            ""
        } else if word.ends_with('/') {
            ""
        } else {
            word.rsplit_once('/').map(|(_, p)| p).unwrap_or(word)
        };

        let show_hidden = filename_prefix.starts_with('.');

        // Read directory and collect matches
        let entries = match std::fs::read_dir(&search_path) {
            Ok(entries) => entries,
            Err(_) => return Vec::new(),
        };

        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Skip hidden files unless prefix starts with '.'
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

    #[test]
    fn test_complete_partial_filename() {
        let dir = setup_test_dir();
        let mut c = FileCompleter::with_cwd(dir.path().to_path_buf());
        let results = c.complete("cat Car", 7);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"Cargo.lock"), "expected Cargo.lock in {values:?}");
        assert!(values.contains(&"Cargo.toml"), "expected Cargo.toml in {values:?}");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_complete_directory_contents() {
        let dir = setup_test_dir();
        let mut c = FileCompleter::with_cwd(dir.path().to_path_buf());
        let results = c.complete("ls src/", 7);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&"src/lib.rs"), "expected src/lib.rs in {values:?}");
        assert!(values.contains(&"src/main.rs"), "expected src/main.rs in {values:?}");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_complete_directory_trailing_slash() {
        let dir = setup_test_dir();
        let mut c = FileCompleter::with_cwd(dir.path().to_path_buf());
        let results = c.complete("cd sr", 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "src/");
        assert!(!results[0].append_whitespace);
    }

    #[test]
    fn test_complete_file_appends_space() {
        let dir = setup_test_dir();
        let mut c = FileCompleter::with_cwd(dir.path().to_path_buf());
        let results = c.complete("cat notes", 9);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "notes.txt");
        assert!(results[0].append_whitespace);
    }

    #[test]
    fn test_hidden_files_excluded() {
        let dir = setup_test_dir();
        let mut c = FileCompleter::with_cwd(dir.path().to_path_buf());
        let results = c.complete("ls C", 4);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        for v in &values {
            assert!(!v.starts_with('.'), "hidden file {v} should be excluded");
        }
    }

    #[test]
    fn test_hidden_files_included_with_dot_prefix() {
        let dir = setup_test_dir();
        let mut c = FileCompleter::with_cwd(dir.path().to_path_buf());
        let results = c.complete("ls .", 4);
        let values: Vec<&str> = results.iter().map(|s| s.value.as_str()).collect();
        assert!(values.contains(&".gitignore"), "expected .gitignore in {values:?}");
        assert!(values.contains(&".hidden_dir/"), "expected .hidden_dir/ in {values:?}");
    }

    #[test]
    fn test_no_matches() {
        let dir = setup_test_dir();
        let mut c = FileCompleter::with_cwd(dir.path().to_path_buf());
        let results = c.complete("cat zzz", 7);
        assert!(results.is_empty());
    }

    #[test]
    fn test_sort_order() {
        let dir = setup_test_dir();
        let mut c = FileCompleter::with_cwd(dir.path().to_path_buf());
        // Complete with no prefix filter — use a single char that matches both dirs and files
        // All non-hidden entries: Cargo.lock, Cargo.toml, notes.txt, src/
        let results = c.complete("ls C", 4);
        // Should only match Cargo.lock and Cargo.toml (files, alphabetical)
        assert_eq!(results[0].value, "Cargo.lock");
        assert_eq!(results[1].value, "Cargo.toml");

        // Now test with a prefix that matches both a dir and files
        // Use "s" which matches "src/"
        let results = c.complete("ls s", 4);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "src/");

        // Broader test: complete everything non-hidden
        // We need a prefix that matches dirs and files — use empty-ish approach
        // Actually let's create a scenario where we can verify ordering
        let root = dir.path();
        fs::write(root.join("aaa_file.txt"), "").unwrap();
        fs::create_dir(root.join("aaa_dir")).unwrap();
        let results = c.complete("ls aaa", 6);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].value, "aaa_dir/"); // dir first
        assert_eq!(results[1].value, "aaa_file.txt"); // then file
    }

    #[test]
    fn test_tilde_expansion() {
        // This test verifies tilde handling — we can't control $HOME in tests,
        // but we can verify the completer returns suggestions starting with ~/
        let mut c = FileCompleter::new();
        let results = c.complete("ls ~/", 4);
        for s in &results {
            assert!(
                s.value.starts_with("~/"),
                "expected suggestion to start with ~/, got: {}",
                s.value
            );
        }
    }
}
