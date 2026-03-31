use nu_ansi_term::{Color, Style};
use nu_color_config::get_shape_color;
use nu_protocol::Config;
use reedline::{Highlighter, StyledText};
use tree_sitter::{Node, Parser};

/// Syntax highlighter for bash using tree-sitter-bash.
/// Colors are read from nushell's color_config so bash highlighting
/// matches the user's nushell theme.
pub struct BashHighlighter {
    keyword: Style,
    command: Style,
    string: Style,
    number: Style,
    variable: Style,
    operator: Style,
    comment: Style,
    foreground: Style,
}

impl BashHighlighter {
    pub fn new(config: &Config) -> Self {
        BashHighlighter {
            keyword: get_shape_color("shape_keyword", config),
            command: get_shape_color("shape_external", config),
            string: get_shape_color("shape_string", config),
            number: get_shape_color("shape_int", config),
            variable: get_shape_color("shape_variable", config),
            operator: get_shape_color("shape_operator", config),
            comment: Style::new().fg(Color::DarkGray),
            foreground: Style::default(),
        }
    }

    fn bash_style(&self, node: &Node) -> Style {
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
}

impl Highlighter for BashHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        let mut styled = StyledText::new();

        if line.is_empty() {
            return styled;
        }

        let language: tree_sitter::Language = tree_sitter_bash::LANGUAGE.into();
        let mut parser = Parser::new();
        if parser.set_language(&language).is_err() {
            styled.push((self.foreground, line.to_string()));
            return styled;
        }

        let tree = match parser.parse(line, None) {
            Some(tree) => tree,
            None => {
                styled.push((self.foreground, line.to_string()));
                return styled;
            }
        };

        let mut segments: Vec<(usize, usize, Style)> = Vec::new();
        collect_leaf_styles(&tree.root_node(), self, &mut segments);

        segments.sort_by_key(|s| s.0);

        let mut pos = 0;
        for (start, end, style) in &segments {
            let start = *start;
            let end = (*end).min(line.len());
            if start > pos {
                styled.push((self.foreground, line[pos..start].to_string()));
            }
            if start >= pos && end > start {
                styled.push((*style, line[start..end].to_string()));
                pos = end;
            }
        }
        if pos < line.len() {
            styled.push((self.foreground, line[pos..].to_string()));
        }

        styled
    }
}

fn collect_leaf_styles(
    node: &Node,
    highlighter: &BashHighlighter,
    segments: &mut Vec<(usize, usize, Style)>,
) {
    if node.child_count() == 0 {
        let start = node.start_byte();
        let end = node.end_byte();
        let style = highlighter.bash_style(node);
        segments.push((start, end, style));
    } else {
        let parent_style = match node.kind() {
            "command_name" => Some(highlighter.command),
            "simple_expansion" | "expansion" => Some(highlighter.variable),
            "string" => Some(highlighter.string),
            _ => None,
        };

        if let Some(style) = parent_style {
            segments.push((node.start_byte(), node.end_byte(), style));
        } else {
            for child in node.children(&mut node.walk()) {
                collect_leaf_styles(&child, highlighter, segments);
            }
        }
    }
}
