use nu_ansi_term::{Color, Style};
use reedline::{Highlighter, StyledText};
use tree_sitter::{Node, Parser};

/// Syntax highlighter for bash using tree-sitter-bash.
pub struct BashHighlighter {
    keyword: Color,
    command: Color,
    string: Color,
    number: Color,
    variable: Color,
    operator: Color,
    comment: Color,
    foreground: Color,
}

impl BashHighlighter {
    pub fn new() -> Self {
        // Tokyo Night colors (sensible defaults)
        BashHighlighter {
            keyword: Color::Rgb(187, 154, 247),  // purple
            command: Color::Rgb(125, 207, 255),   // blue
            string: Color::Rgb(158, 206, 106),    // green
            number: Color::Rgb(255, 158, 100),    // orange
            variable: Color::Rgb(224, 175, 104),  // yellow
            operator: Color::Rgb(137, 221, 255),  // cyan
            comment: Color::Rgb(86, 95, 137),     // gray
            foreground: Color::Rgb(192, 202, 245), // light gray
        }
    }

    fn bash_color(&self, node: &Node) -> Color {
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
            styled.push((Style::new().fg(self.foreground), line.to_string()));
            return styled;
        }

        let tree = match parser.parse(line, None) {
            Some(tree) => tree,
            None => {
                styled.push((Style::new().fg(self.foreground), line.to_string()));
                return styled;
            }
        };

        let mut segments: Vec<(usize, usize, Color)> = Vec::new();
        collect_leaf_styles(&tree.root_node(), self, &mut segments);

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
    highlighter: &BashHighlighter,
    segments: &mut Vec<(usize, usize, Color)>,
) {
    if node.child_count() == 0 {
        let start = node.start_byte();
        let end = node.end_byte();
        let color = highlighter.bash_color(node);
        segments.push((start, end, color));
    } else {
        // Handle parent nodes that should color all children
        let parent_color = match node.kind() {
            "command_name" => Some(highlighter.command),
            "simple_expansion" | "expansion" => Some(highlighter.variable),
            "string" => Some(highlighter.string),
            _ => None,
        };

        if let Some(color) = parent_color {
            segments.push((node.start_byte(), node.end_byte(), color));
        } else {
            for child in node.children(&mut node.walk()) {
                collect_leaf_styles(&child, highlighter, segments);
            }
        }
    }
}
