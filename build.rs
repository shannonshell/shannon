use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    // --- Completions ---
    build_completions(&out_dir);

    // --- Themes ---
    build_themes(&out_dir);
}

fn build_completions(out_dir: &str) {
    let completions_dir = Path::new("completions");
    println!("cargo:rerun-if-changed=completions/");

    if !completions_dir.exists() {
        fs::write(Path::new(out_dir).join("completions.json"), "{}").unwrap();
        return;
    }

    let mut table: HashMap<String, CommandSpec> = HashMap::new();

    for entry in fs::read_dir(completions_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map(|e| e == "fish").unwrap_or(false) {
            let contents = fs::read_to_string(&path).unwrap_or_default();
            parse_fish_completions(&contents, &mut table);
        }
    }

    let json = serde_json::to_string(&table).unwrap();
    fs::write(Path::new(out_dir).join("completions.json"), json).unwrap();

    eprintln!(
        "build.rs: parsed {} commands from fish completions",
        table.len()
    );
}

/// Fish variable → shannon semantic category mapping.
fn map_fish_var(var: &str) -> Option<&'static str> {
    match var {
        "fish_color_command" => Some("command"),
        "fish_color_keyword" => Some("keyword"),
        "fish_color_quote" => Some("string"),
        "fish_color_comment" => Some("comment"),
        "fish_color_error" => Some("error"),
        "fish_color_normal" => Some("foreground"),
        "fish_color_autosuggestion" => Some("hint"),
        "fish_color_redirection" => Some("operator"),
        "fish_color_option" => Some("operator"),
        "fish_color_operator" => Some("operator"),
        "fish_color_escape" => Some("variable"),
        "fish_color_param" => Some("foreground"),
        "fish_pager_color_description" => Some("menu_description"),
        "fish_pager_color_prefix" => Some("menu_match"),
        "fish_pager_color_completion" => Some("menu_text"),
        _ => None,
    }
}

/// Parse a fish .theme file into sections with mapped shannon categories.
fn parse_fish_theme(
    contents: &str,
) -> HashMap<String, HashMap<String, String>> {
    let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current_section = "unknown".to_string();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            continue;
        }
        // Lines: fish_color_name value [--modifiers]
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() < 2 {
            continue;
        }
        let var_name = parts[0];
        let color_value = parts[1].trim();

        if let Some(category) = map_fish_var(var_name) {
            // Normalize hex colors: bare "ab1234" → "#ab1234"
            let normalized = if color_value.len() >= 6
                && !color_value.starts_with('#')
                && !color_value.starts_with('-')
                && color_value[..6].chars().all(|c| c.is_ascii_hexdigit())
            {
                format!("#{color_value}")
            } else {
                color_value.to_string()
            };

            sections
                .entry(current_section.clone())
                .or_default()
                .insert(category.to_string(), normalized);
        }
    }

    sections
}

fn build_themes(out_dir: &str) {
    let themes_dir = Path::new("themes");
    println!("cargo:rerun-if-changed=themes/");

    if !themes_dir.exists() {
        fs::write(Path::new(out_dir).join("themes.json"), "{}").unwrap();
        return;
    }

    let mut all_themes: HashMap<String, HashMap<String, HashMap<String, String>>> = HashMap::new();

    for entry in fs::read_dir(themes_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map(|e| e == "theme").unwrap_or(false) {
            let name = path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();
            let contents = fs::read_to_string(&path).unwrap_or_default();
            let sections = parse_fish_theme(&contents);
            all_themes.insert(name, sections);
        }
    }

    let json = serde_json::to_string(&all_themes).unwrap();
    fs::write(Path::new(out_dir).join("themes.json"), json).unwrap();

    eprintln!("build.rs: parsed {} themes", all_themes.len());
}

#[derive(Default, serde::Serialize)]
struct CommandSpec {
    subcommands: Vec<CompletionEntry>,
    global_flags: Vec<FlagEntry>,
    subcommand_flags: HashMap<String, Vec<FlagEntry>>,
}

#[derive(serde::Serialize)]
struct CompletionEntry {
    name: String,
    description: String,
}

#[derive(serde::Serialize)]
struct FlagEntry {
    short: Option<char>,
    long: Option<String>,
    description: String,
    takes_arg: bool,
}

fn parse_fish_completions(contents: &str, table: &mut HashMap<String, CommandSpec>) {
    for line in contents.lines() {
        let line = line.trim();
        if !line.starts_with("complete ") {
            continue;
        }
        if let Some(parsed) = parse_complete_line(line) {
            let spec = table.entry(parsed.command.clone()).or_default();
            apply_parsed(spec, parsed);
        }
    }
}

struct ParsedComplete {
    command: String,
    short: Option<char>,
    long: Option<String>,
    args: Option<String>,
    description: String,
    _no_file: bool,
    takes_arg: bool,
    condition: Condition,
}

enum Condition {
    None,
    NeedsSubcommand,
    SeenSubcommand(Vec<String>),
    Other,
}

fn parse_complete_line(line: &str) -> Option<ParsedComplete> {
    let tokens = tokenize(line);
    if tokens.is_empty() || tokens[0] != "complete" {
        return None;
    }

    let mut command = String::new();
    let mut short = None;
    let mut long = None;
    let mut args = None;
    let mut description = String::new();
    let mut _no_file = false;
    let mut takes_arg = false;
    let mut condition = Condition::None;

    // Check if the second token is a positional command name (not a flag)
    // e.g. "complete git -f -n ..." instead of "complete -c git -f -n ..."
    if tokens.len() > 1 && !tokens[1].starts_with('-') {
        command = tokens[1].clone();
    }

    let mut i = 1;
    while i < tokens.len() {
        match tokens[i].as_str() {
            "-c" | "--command" => {
                i += 1;
                if i < tokens.len() {
                    command = tokens[i].clone();
                }
            }
            "-s" | "--short-option" => {
                i += 1;
                if i < tokens.len() {
                    short = tokens[i].chars().next();
                }
            }
            "-l" | "--long-option" => {
                i += 1;
                if i < tokens.len() {
                    long = Some(tokens[i].clone());
                }
            }
            "-o" | "--old-option" => {
                // Old-style option (single dash, multi-char) — treat like long
                i += 1;
                if i < tokens.len() {
                    long = Some(tokens[i].clone());
                }
            }
            "-a" | "--arguments" => {
                i += 1;
                if i < tokens.len() {
                    args = Some(tokens[i].clone());
                }
            }
            "-d" | "--description" => {
                i += 1;
                if i < tokens.len() {
                    description = tokens[i].clone();
                }
            }
            "-f" | "--no-files" => {
                _no_file = true;
            }
            "-r" | "--require-parameter" => {
                takes_arg = true;
            }
            "-x" | "--exclusive" => {
                _no_file = true;
                takes_arg = true;
            }
            "-F" | "--force-files" => {}
            "-k" | "--keep-order" => {}
            "-w" | "--wraps" => {
                i += 1; // skip value
            }
            "-e" | "--erase" => {
                return None; // erase command, skip
            }
            "-n" | "--condition" => {
                i += 1;
                if i < tokens.len() {
                    condition = parse_condition(&tokens[i]);
                }
            }
            _ => {}
        }
        i += 1;
    }

    if command.is_empty() {
        return None;
    }

    Some(ParsedComplete {
        command,
        short,
        long,
        args,
        description,
        _no_file,
        takes_arg,
        condition,
    })
}

fn parse_condition(cond: &str) -> Condition {
    // Strip outer quotes if present
    let stripped = cond
        .trim()
        .trim_start_matches('\'')
        .trim_end_matches('\'')
        .trim_start_matches('"')
        .trim_end_matches('"')
        .trim();

    // Generic: __fish_use_subcommand
    if stripped == "__fish_use_subcommand" {
        return Condition::NeedsSubcommand;
    }

    // Generic: __fish_seen_subcommand_from X Y Z
    if let Some(rest) = stripped.strip_prefix("__fish_seen_subcommand_from ") {
        let subs: Vec<String> = rest.split_whitespace().map(|s| s.to_string()).collect();
        if !subs.is_empty() {
            return Condition::SeenSubcommand(subs);
        }
    }

    // Tool-specific: __fish_<tool>_needs_command (equivalent to __fish_use_subcommand)
    if stripped.starts_with("__fish_")
        && stripped.ends_with("_needs_command")
        && !stripped.contains(' ')
    {
        return Condition::NeedsSubcommand;
    }

    // Tool-specific: __fish_<tool>_using_command X Y Z (equivalent to __fish_seen_subcommand_from)
    for prefix in ["_using_command ", "_is_using_command "] {
        if let Some(pos) = stripped.find(prefix) {
            if stripped.starts_with("__fish_") {
                let rest = &stripped[pos + prefix.len()..];
                let subs: Vec<String> =
                    rest.split_whitespace().map(|s| s.to_string()).collect();
                if !subs.is_empty() {
                    return Condition::SeenSubcommand(subs);
                }
            }
        }
    }

    // Also handle: __fish_<tool>_using_command (no args = any subcommand active)
    // and conditions with semicolons (compound conditions) — skip these
    if stripped.contains(';') || stripped.contains("&&") || stripped.contains("||") {
        return Condition::Other;
    }

    Condition::Other
}

fn apply_parsed(spec: &mut CommandSpec, p: ParsedComplete) {
    let is_flag = p.short.is_some() || p.long.is_some();

    match p.condition {
        Condition::Other => return, // skip conditions we can't evaluate
        Condition::NeedsSubcommand => {
            if is_flag {
                spec.global_flags.push(FlagEntry {
                    short: p.short,
                    long: p.long,
                    description: p.description.clone(),
                    takes_arg: p.takes_arg,
                });
            }
            if let Some(args) = &p.args {
                for arg in split_args(args) {
                    if !arg.starts_with('(') && !arg.contains('$') {
                        spec.subcommands.push(CompletionEntry {
                            name: arg.clone(),
                            description: p.description.clone(),
                        });
                    }
                }
            }
        }
        Condition::SeenSubcommand(subs) => {
            let flag = if is_flag {
                Some(FlagEntry {
                    short: p.short,
                    long: p.long,
                    description: p.description.clone(),
                    takes_arg: p.takes_arg,
                })
            } else {
                None
            };
            for sub in subs {
                if let Some(flag) = &flag {
                    spec.subcommand_flags
                        .entry(sub)
                        .or_default()
                        .push(FlagEntry {
                            short: flag.short,
                            long: flag.long.clone(),
                            description: flag.description.clone(),
                            takes_arg: flag.takes_arg,
                        });
                }
            }
        }
        Condition::None => {
            // No condition — global flag or unconditional arg
            if is_flag {
                spec.global_flags.push(FlagEntry {
                    short: p.short,
                    long: p.long,
                    description: p.description.clone(),
                    takes_arg: p.takes_arg,
                });
            }
            if let Some(args) = &p.args {
                for arg in split_args(args) {
                    if !arg.starts_with('(') && !arg.contains('$') {
                        spec.subcommands.push(CompletionEntry {
                            name: arg.clone(),
                            description: p.description.clone(),
                        });
                    }
                }
            }
        }
    }
}

/// Split a fish `-a` argument value into individual tokens.
/// Handles space-separated values and tab-separated value\tdescription pairs.
fn split_args(args: &str) -> Vec<String> {
    args.split_whitespace()
        .map(|s| {
            // Strip tab-separated description if present
            s.split('\t').next().unwrap_or(s).to_string()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// Tokenize a fish `complete` command line, handling quoted strings.
fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while let Some(c) = chars.next() {
        if in_single_quote {
            if c == '\'' {
                in_single_quote = false;
            } else {
                current.push(c);
            }
        } else if in_double_quote {
            if c == '"' {
                in_double_quote = false;
            } else if c == '\\' {
                if let Some(&next) = chars.peek() {
                    if next == '"' || next == '\\' {
                        current.push(chars.next().unwrap());
                    } else {
                        current.push(c);
                    }
                }
            } else {
                current.push(c);
            }
        } else if c == '\'' {
            in_single_quote = true;
        } else if c == '"' {
            in_double_quote = true;
        } else if c.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}
