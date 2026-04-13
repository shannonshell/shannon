use nu_test_support::nu;

#[test]
fn md_empty() {
    let actual = nu!(r#"
            echo [[]; []] | from json | to md
        "#);

    assert_eq!(actual.out, "");
}

#[test]
fn md_empty_pretty() {
    let actual = nu!(r#"
            echo "{}" | from json | to md -p
        "#);

    assert_eq!(actual.out, "");
}

#[test]
fn md_simple() {
    let actual = nu!(r#"
            echo 3 | to md
        "#);

    assert_eq!(actual.out, "* 3");
}

#[test]
fn md_simple_pretty() {
    let actual = nu!(r#"
            echo 3 | to md -p
        "#);

    assert_eq!(actual.out, "* 3");
}

#[test]
fn md_table() {
    let actual = nu!(r#"
            echo [[name]; [jason]] | to md
        "#);

    assert_eq!(actual.out, "| name || --- || jason |");
}

#[test]
fn md_table_pretty() {
    let actual = nu!(r#"
            echo [[name]; [joseph]] | to md -p
        "#);

    assert_eq!(actual.out, "| name   || ------ || joseph |");
}

#[test]
fn md_combined() {
    let actual = nu!(r#"
        def title [] {
            echo [[H1]; ["Nu top meals"]]
        };

        def meals [] {
            echo [[dish]; [Arepa] [Taco] [Pizza]]
        };

        title
        | append (meals)
        | to md --per-element --pretty
    "#);

    assert_eq!(
        actual.out,
        "# Nu top meals| dish  || ----- || Arepa || Taco  || Pizza |"
    );
}

#[test]
fn from_md_ast_first_node_type() -> Result {
    let code = "'# Title' | from md | get 0.type";

    test().run(code).expect_value_eq("h1")
}

#[test]
fn from_md_ast_frontmatter_node() -> Result {
    let code = "'---
title: Demo
---
# Heading' | from md | get 0.type";

    test().run(code).expect_value_eq("yaml")
}

#[test]
fn from_md_ast_has_position() -> Result {
    let code = "'# Title' | from md | get 0.position.start.line";

    test().run(code).expect_value_eq(1)
}

#[test]
fn from_md_ast_preserves_interline_text_value() -> Result {
    let code = r#""[a](https://a)
[b](https://b)" | from md | get 1.attrs.value | str length"#;

    test().run(code).expect_value_eq(1)
}
