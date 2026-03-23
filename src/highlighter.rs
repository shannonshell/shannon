use nu_ansi_term::{Color, Style};
use reedline::{Highlighter, StyledText};
use tree_sitter::{Language, Node, Parser};

use crate::theme::Theme;

pub struct TreeSitterHighlighter {
    grammar: String,
    keyword: Color,
    command: Color,
    string: Color,
    number: Color,
    variable: Color,
    operator: Color,
    comment: Color,
    error: Color,
    foreground: Color,
    type_: Color,
}

/// Extract the foreground color from a Style, falling back to White.
fn fg_color(style: &Style) -> Color {
    style.foreground.unwrap_or(Color::White)
}

impl TreeSitterHighlighter {
    pub fn new(highlighter: Option<&str>, theme: &Theme) -> Self {
        TreeSitterHighlighter {
            grammar: highlighter.unwrap_or("").to_string(),
            keyword: fg_color(&theme.keyword),
            command: fg_color(&theme.command),
            string: fg_color(&theme.string),
            number: fg_color(&theme.number),
            variable: fg_color(&theme.variable),
            operator: fg_color(&theme.operator),
            comment: fg_color(&theme.comment),
            error: fg_color(&theme.error),
            foreground: fg_color(&theme.foreground),
            type_: fg_color(&theme.type_),
        }
    }

    fn make_parser(&self) -> Option<Parser> {
        let language: Language = match self.grammar.as_str() {
            "bash" => tree_sitter_bash::LANGUAGE.into(),
            "nushell" => crate::tree_sitter_nu::LANGUAGE.into(),
            "fish" => tree_sitter_fish::language(),
            _ => return None,
        };
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .expect("failed to set language");
        Some(parser)
    }

    fn style_for_node(&self, node: &Node, source: &str) -> Color {
        let kind = node.kind();

        if kind == "ERROR" || kind == "MISSING" {
            return self.error;
        }

        match self.grammar.as_str() {
            "bash" => self.bash_color(node, source),
            "nushell" => self.nushell_color(node, source),
            "fish" => self.fish_color(node, source),
            _ => self.foreground,
        }
    }

    fn bash_color(&self, node: &Node, _source: &str) -> Color {
        match node.kind() {
            "if" | "then" | "else" | "elif" | "fi" | "for" | "in" | "do" | "done" | "while"
            | "until" | "case" | "esac" | "function" | "export" | "declare" | "local"
            | "return" | "select" => self.keyword,

            "command_name" => self.command,

            "string" | "raw_string" | "heredoc_body" | "string_content" | "ansii_c_string" => {
                self.string
            }

            "number" => self.number,

            "variable_name" | "special_variable_name" => self.variable,
            "simple_expansion" | "expansion" => self.variable,
            "$" => self.variable,

            "|" | ">" | ">>" | "<" | "<<" | "&&" | "||" | ";" | ";;" | "&" => self.operator,
            "test_operator" => self.operator,

            "comment" => self.comment,

            _ => self.foreground,
        }
    }

    fn nushell_color(&self, node: &Node, _source: &str) -> Color {
        match node.kind() {
            "if" | "else" | "for" | "in" | "let" | "mut" | "def" | "where" | "match" | "while"
            | "loop" | "break" | "continue" | "return" | "try" | "catch" | "export" | "use"
            | "module" | "overlay" | "source" | "hide" | "const" => self.keyword,

            "cmd_identifier" => self.command,

            "val_string" | "string_content" | "escaped_interpolated_content" => self.string,
            "'" | "\"" | "$\"" | "$'" => self.string,

            "val_number" => self.number,
            "val_bool" | "true" | "false" => self.number,

            "val_variable" | "identifier" => self.variable,
            "$" => self.variable,

            "flat_type" | "param_type" => self.type_,

            "|" | ">" | "<" | ">=" | "<=" | "==" | "!=" | "=" | "+" | "-" | "*" | "/" | ".."
            | "..." | "=~" | "!~" | "and" | "or" | "not" => self.operator,

            "comment" => self.comment,

            "filesize_unit" | "duration_unit" => self.type_,

            _ => self.foreground,
        }
    }

    fn fish_color(&self, node: &Node, _source: &str) -> Color {
        match node.kind() {
            "if" | "else" | "else_if" | "for" | "in" | "while" | "switch" | "case"
            | "function" | "end" | "begin" | "return" | "and" | "or" | "not" | "break"
            | "continue" | "set" | "builtin" | "command" | "exec" | "source" => self.keyword,

            "single_quote_string" | "double_quote_string" | "escape_sequence" => self.string,

            "integer" | "float" => self.number,

            "variable_name" | "variable_expansion" => self.variable,
            "$" => self.variable,

            "|" | ">" | ">>" | "<" | "&" | "&&" | "||" | ";" => self.operator,
            "pipe" | "direction" => self.operator,

            "comment" => self.comment,

            "glob" | "home_dir_expansion" => self.type_,

            _ => self.foreground,
        }
    }
}

impl Highlighter for TreeSitterHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        let mut styled = StyledText::new();

        if line.is_empty() {
            return styled;
        }

        let mut parser = match self.make_parser() {
            Some(p) => p,
            None => {
                styled.push((Style::new().fg(self.foreground), line.to_string()));
                return styled;
            }
        };

        let tree = match parser.parse(line, None) {
            Some(tree) => tree,
            None => {
                styled.push((Style::new().fg(self.foreground), line.to_string()));
                return styled;
            }
        };

        let mut segments: Vec<(usize, usize, Color)> = Vec::new();
        collect_leaf_styles(&tree.root_node(), line, self, &mut segments);

        segments.sort_by_key(|s| s.0);

        let mut pos = 0;
        for (start, end, color) in &segments {
            let start = *start;
            let end = (*end).min(line.len());
            if start > pos {
                styled.push((Style::new().fg(self.foreground), line[pos..start].to_string()));
            }
            if start >= pos && end > start {
                styled.push((Style::new().fg(*color), line[start..end].to_string()));
                pos = end;
            }
        }
        if pos < line.len() {
            styled.push((Style::new().fg(self.foreground), line[pos..].to_string()));
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
        let start = node.start_byte();
        let end = node.end_byte();
        let color = highlighter.style_for_node(node, source);
        segments.push((start, end, color));
    } else {
        let parent_color = match highlighter.grammar.as_str() {
            "bash" => {
                if node.kind() == "command_name" {
                    Some(highlighter.command)
                } else if node.kind() == "simple_expansion" || node.kind() == "expansion" {
                    Some(highlighter.variable)
                } else if node.kind() == "string" {
                    Some(highlighter.string)
                } else {
                    None
                }
            }
            "nushell" => {
                if node.kind() == "val_string" {
                    Some(highlighter.string)
                } else if node.kind() == "val_variable" {
                    Some(highlighter.variable)
                } else {
                    None
                }
            }
            "fish" => {
                if node.kind() == "double_quote_string"
                    || node.kind() == "single_quote_string"
                {
                    Some(highlighter.string)
                } else if node.kind() == "variable_expansion" {
                    Some(highlighter.variable)
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(color) = parent_color {
            segments.push((node.start_byte(), node.end_byte(), color));
        } else if highlighter.grammar == "fish" && node.kind() == "command" {
            let mut first = true;
            for child in node.children(&mut node.walk()) {
                if first && child.kind() == "word" {
                    segments.push((child.start_byte(), child.end_byte(), highlighter.command));
                    first = false;
                } else {
                    first = false;
                    collect_leaf_styles(&child, source, highlighter, segments);
                }
            }
        } else {
            for child in node.children(&mut node.walk()) {
                collect_leaf_styles(&child, source, highlighter, segments);
            }
        }
    }
}
