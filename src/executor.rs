use std::collections::HashMap;
use std::path::PathBuf;

/// Parse bash `export -p` output plus __SHANNON_ markers.
pub fn parse_bash_env(contents: &str) -> Option<(HashMap<String, String>, PathBuf)> {
    let mut env = HashMap::new();
    let mut cwd: Option<PathBuf> = None;

    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("declare -x ") {
            if let Some((key, value)) = parse_declare_line(rest) {
                if key == "__SHANNON_CWD" {
                    cwd = Some(PathBuf::from(&value));
                } else if key == "__SHANNON_EXIT" {
                    // Skip
                } else {
                    env.insert(key, value);
                }
            }
        } else if let Some(rest) = line.strip_prefix("__SHANNON_CWD=") {
            cwd = Some(PathBuf::from(rest));
        } else if line.starts_with("__SHANNON_EXIT=") {
            // Skip
        }
    }

    Some((env, cwd.unwrap_or_else(|| PathBuf::from("/"))))
}

fn parse_declare_line(s: &str) -> Option<(String, String)> {
    if let Some(eq_pos) = s.find('=') {
        let key = s[..eq_pos].to_string();
        let raw_value = &s[eq_pos + 1..];
        let value =
            if raw_value.starts_with('"') && raw_value.ends_with('"') && raw_value.len() >= 2 {
                unescape_bash_value(&raw_value[1..raw_value.len() - 1])
            } else {
                raw_value.to_string()
            };
        Some((key, value))
    } else {
        None
    }
}

fn unescape_bash_value(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some('$') => result.push('$'),
                Some('`') => result.push('`'),
                Some('\n') => {}
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}
