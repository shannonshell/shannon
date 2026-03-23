//! Vendored tree-sitter grammar for nushell.
//! Source: https://github.com/nushell/tree-sitter-nu (MIT license)

use tree_sitter_language::LanguageFn;

extern "C" {
    fn tree_sitter_nu() -> *const ();
}

/// The tree-sitter [`LanguageFn`] for the nushell grammar.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_nu) };
