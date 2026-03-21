use nu_ansi_term::{Color, Style};
use reedline::{Highlighter, StyledText};
use tree_sitter::{Language, Node, Parser};

use crate::shell::ShellKind;

// Tokyo Night color palette
const PURPLE: Color = Color::Rgb(187, 154, 247); // #bb9af7 — keywords
const BLUE: Color = Color::Rgb(122, 162, 247); // #7aa2f7 — commands
const GREEN: Color = Color::Rgb(158, 206, 106); // #9ece6a — strings
const ORANGE: Color = Color::Rgb(255, 158, 100); // #ff9e64 — numbers, booleans
const CYAN: Color = Color::Rgb(125, 207, 255); // #7dcfff — variables
const YELLOW: Color = Color::Rgb(224, 175, 104); // #e0af68 — types
const GRAY: Color = Color::Rgb(86, 95, 137); // #565f89 — comments
const RED: Color = Color::Rgb(247, 118, 142); // #f7768e — errors
const FG: Color = Color::Rgb(169, 177, 214); // #a9b1d6 — default foreground
const OPERATOR: Color = Color::Rgb(137, 221, 255); // #89ddff — operators/pipes

pub struct TreeSitterHighlighter {
    shell: ShellKind,
}

impl TreeSitterHighlighter {
    pub fn new(shell: ShellKind) -> Self {
        TreeSitterHighlighter { shell }
    }

    fn make_parser(&self) -> Parser {
        let mut parser = Parser::new();
        let language: Language = match self.shell {
            ShellKind::Bash => tree_sitter_bash::LANGUAGE.into(),
            ShellKind::Nushell => tree_sitter_nu::LANGUAGE.into(),
        };
        parser
            .set_language(&language)
            .expect("failed to set language");
        parser
    }

    fn style_for_node(&self, node: &Node, source: &str) -> Color {
        let kind = node.kind();

        // Error nodes
        if kind == "ERROR" || kind == "MISSING" {
            return RED;
        }

        match self.shell {
            ShellKind::Bash => self.bash_color(node, source),
            ShellKind::Nushell => self.nushell_color(node, source),
        }
    }

    fn bash_color(&self, node: &Node, _source: &str) -> Color {
        let kind = node.kind();
        match kind {
            // Keywords
            "if" | "then" | "else" | "elif" | "fi" | "for" | "in" | "do" | "done" | "while"
            | "until" | "case" | "esac" | "function" | "export" | "declare" | "local"
            | "return" | "select" => PURPLE,

            // Command names — the `word` inside a `command_name` node
            "command_name" => BLUE,

            // Strings
            "string" | "raw_string" | "heredoc_body" | "string_content" | "ansii_c_string" => GREEN,

            // Numbers
            "number" => ORANGE,

            // Variables
            "variable_name" | "special_variable_name" => CYAN,
            "simple_expansion" | "expansion" => CYAN,
            "$" => CYAN,

            // Operators and punctuation
            "|" | ">" | ">>" | "<" | "<<" | "&&" | "||" | ";" | ";;" | "&" => OPERATOR,
            "test_operator" => OPERATOR,

            // Comments
            "comment" => GRAY,

            _ => FG,
        }
    }

    fn nushell_color(&self, node: &Node, _source: &str) -> Color {
        let kind = node.kind();
        match kind {
            // Keywords
            "if" | "else" | "for" | "in" | "let" | "mut" | "def" | "where" | "match" | "while"
            | "loop" | "break" | "continue" | "return" | "try" | "catch" | "export" | "use"
            | "module" | "overlay" | "source" | "hide" | "const" => PURPLE,

            // Command identifiers
            "cmd_identifier" => BLUE,

            // Strings
            "val_string" | "string_content" | "escaped_interpolated_content" => GREEN,
            "'" | "\"" | "$\"" | "$'" => GREEN,

            // Numbers and booleans
            "val_number" => ORANGE,
            "val_bool" | "true" | "false" => ORANGE,

            // Variables
            "val_variable" | "identifier" => CYAN,
            "$" => CYAN,

            // Types
            "flat_type" | "param_type" => YELLOW,

            // Operators and pipes
            "|" | ">" | "<" | ">=" | "<=" | "==" | "!=" | "=" | "+" | "-" | "*" | "/" | ".."
            | "..." | "=~" | "!~" | "and" | "or" | "not" => OPERATOR,

            // Comments
            "comment" => GRAY,

            // Filesize/duration units
            "filesize_unit" | "duration_unit" => YELLOW,

            _ => FG,
        }
    }
}

impl Highlighter for TreeSitterHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        let mut styled = StyledText::new();

        if line.is_empty() {
            return styled;
        }

        let mut parser = self.make_parser();

        let tree = match parser.parse(line, None) {
            Some(tree) => tree,
            None => {
                // Parse failed — return unstyled
                styled.push((Style::new().fg(FG), line.to_string()));
                return styled;
            }
        };

        // Collect leaf nodes with their byte ranges and colors
        let mut segments: Vec<(usize, usize, Color)> = Vec::new();
        collect_leaf_styles(&tree.root_node(), line, self, &mut segments);

        // Sort by start position
        segments.sort_by_key(|s| s.0);

        // Build styled text, filling gaps with default color
        let mut pos = 0;
        for (start, end, color) in &segments {
            let start = *start;
            let end = (*end).min(line.len());
            if start > pos {
                // Gap before this segment — default color
                styled.push((Style::new().fg(FG), line[pos..start].to_string()));
            }
            if start >= pos && end > start {
                styled.push((Style::new().fg(*color), line[start..end].to_string()));
                pos = end;
            }
        }
        // Trailing content
        if pos < line.len() {
            styled.push((Style::new().fg(FG), line[pos..].to_string()));
        }

        styled
    }
}

fn collect_leaf_styles(
    node: &Node,
    source: &str,
    highlighter: &TreeSitterHighlighter,
    segments: &mut Vec<(usize, usize, Color)>,
) {
    if node.child_count() == 0 {
        // Leaf node
        let start = node.start_byte();
        let end = node.end_byte();
        let color = highlighter.style_for_node(node, source);
        segments.push((start, end, color));
    } else {
        // For named parent nodes that map to a color (like command_name),
        // color all their children with the parent's color
        let parent_color = match highlighter.shell {
            ShellKind::Bash => {
                if node.kind() == "command_name" {
                    Some(BLUE)
                } else if node.kind() == "simple_expansion" || node.kind() == "expansion" {
                    Some(CYAN)
                } else if node.kind() == "string" {
                    Some(GREEN)
                } else {
                    None
                }
            }
            ShellKind::Nushell => {
                if node.kind() == "val_string" {
                    Some(GREEN)
                } else if node.kind() == "val_variable" {
                    Some(CYAN)
                } else {
                    None
                }
            }
        };

        if let Some(color) = parent_color {
            // Color the entire span of this node
            segments.push((node.start_byte(), node.end_byte(), color));
        } else {
            // Recurse into children
            for child in node.children(&mut node.walk()) {
                collect_leaf_styles(&child, source, highlighter, segments);
            }
        }
    }
}
