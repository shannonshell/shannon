use std::collections::BTreeSet;
use tree_sitter::{Language, Node, Parser};

fn collect_node_types(node: Node, types: &mut BTreeSet<String>) {
    types.insert(node.kind().to_string());
    for child in node.children(&mut node.walk()) {
        collect_node_types(child, types);
    }
}

fn parse_and_dump(parser: &mut Parser, label: &str, input: &str) {
    println!("--- {label} ---");
    println!("Input: {input:?}");

    match parser.parse(input, None) {
        Some(tree) => {
            let root = tree.root_node();
            println!("S-expression: {}", root.to_sexp());
            println!("Has errors: {}", root.has_error());
            println!();
        }
        None => {
            println!("PARSE FAILED (returned None)");
            println!();
        }
    }
}

fn run_shell(name: &str, language: Language, samples: &[(&str, &str)]) {
    println!("========================================");
    println!("  {name}");
    println!("========================================\n");

    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .expect("failed to set language");

    let mut all_types = BTreeSet::new();

    for (label, input) in samples {
        parse_and_dump(&mut parser, label, input);

        if let Some(tree) = parser.parse(input, None) {
            collect_node_types(tree.root_node(), &mut all_types);
        }
    }

    println!("--- All node types ---");
    for t in &all_types {
        println!("  {t}");
    }
    println!();
}

fn main() {
    let bash_samples: Vec<(&str, &str)> = vec![
        ("complete command", r#"echo "hello world""#),
        ("pipeline", "ls -la | grep foo"),
        ("variable export", "export FOO=bar"),
        ("variable use", r#"echo $FOO"#),
        ("incomplete string", r#"echo "unterminated"#),
        ("comment", "# this is a comment"),
        ("command substitution", "echo $(date +%Y)"),
        ("if statement", r#"if [ -f foo ]; then echo "yes"; fi"#),
        ("for loop", "for i in 1 2 3; do echo $i; done"),
        ("redirect", "echo hello > output.txt"),
        ("heredoc", "cat <<EOF\nhello\nEOF"),
        ("function def", "my_func() { echo hi; }"),
    ];

    let nu_samples: Vec<(&str, &str)> = vec![
        ("complete command", r#"echo "hello world""#),
        ("pipeline", "ls | where size > 1kb"),
        ("variable assignment", "let foo = 'bar'"),
        ("env variable", r#"$env.FOO = "bar""#),
        ("incomplete string", r#"echo "unterminated"#),
        ("comment", "# this is a comment"),
        ("closure", "{ |x| $x + 1 }"),
        ("if expression", r#"if true { "yes" } else { "no" }"#),
        ("for loop", "for x in [1 2 3] { print $x }"),
        ("string interpolation", r#"$"hello ($name)""#),
        ("range", "1..10"),
        ("record", r#"{ name: "alice", age: 30 }"#),
        ("def command", "def greet [name: string] { print $name }"),
    ];

    run_shell(
        "Bash",
        tree_sitter_bash::LANGUAGE.into(),
        &bash_samples,
    );

    run_shell(
        "Nushell",
        tree_sitter_nu::LANGUAGE.into(),
        &nu_samples,
    );
}
