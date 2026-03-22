use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn main() {
    let completions_dir = Path::new("completions");
    println!("cargo:rerun-if-changed=completions/");

    if !completions_dir.exists() {
        // No completions directory — write empty table
        let out_dir = std::env::var("OUT_DIR").unwrap();
        fs::write(Path::new(&out_dir).join("completions.json"), "{}").unwrap();
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
    let out_dir = std::env::var("OUT_DIR").unwrap();
    fs::write(Path::new(&out_dir).join("completions.json"), json).unwrap();

    eprintln!(
        "build.rs: parsed {} commands from fish completions",
        table.len()
    );
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
    no_file: bool,
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
    let mut no_file = false;
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
                no_file = true;
            }
            "-r" | "--require-parameter" => {
                takes_arg = true;
            }
            "-x" | "--exclusive" => {
                no_file = true;
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
        no_file,
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
