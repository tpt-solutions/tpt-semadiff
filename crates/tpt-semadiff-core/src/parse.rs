//! Tree-sitter front end: parses source text into the arena IR.

use std::fmt;
use std::path::Path;

use crate::ir::ArenaTree;
use crate::tree::{NodeId, Span, Tree};

/// Languages the parser understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLang {
    Rust,
    Python,
    JavaScript,
    Go,
}

impl SourceLang {
    /// Picks a language from a file extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "rs" => Some(Self::Rust),
            "py" => Some(Self::Python),
            "js" | "mjs" | "cjs" | "jsx" => Some(Self::JavaScript),
            "go" => Some(Self::Go),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::Go => "go",
        }
    }

    pub(crate) fn grammar(self) -> tree_sitter::Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
        }
    }

    /// Node kinds that introduce a named function or method.
    pub fn function_kinds(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["function_item"],
            Self::Python => &["function_definition"],
            Self::JavaScript => &["function_declaration", "method_definition"],
            Self::Go => &["function_declaration", "method_declaration"],
        }
    }

    /// Node kinds that represent an import.
    pub fn import_kinds(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["use_declaration"],
            Self::Python => &["import_statement", "import_from_statement"],
            Self::JavaScript => &["import_statement"],
            Self::Go => &["import_declaration", "import_spec"],
        }
    }

    /// Node kinds that bind a variable, for the inline-variable refactoring rule.
    pub fn binding_kinds(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["let_declaration"],
            Self::Python => &["assignment"],
            Self::JavaScript => &["lexical_declaration", "variable_declaration"],
            Self::Go => &["short_var_declaration"],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The grammar could not be loaded, usually an ABI mismatch.
    Grammar(String),
    /// The parser produced no tree at all.
    Failed,
    /// The source contains a syntax error. `line` is the first offending line (1-based).
    Syntax { line: usize },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Grammar(msg) => write!(f, "could not load grammar: {msg}"),
            Self::Failed => write!(f, "parser produced no tree"),
            Self::Syntax { line } => write!(f, "syntax error on line {line}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parses `source` into an [`ArenaTree`] that keeps every token, comments included.
/// Leaves carry their source text as value; inner nodes carry none.
pub fn parse_tree(source: &str, lang: SourceLang) -> Result<ArenaTree, ParseError> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&lang.grammar())
        .map_err(|e| ParseError::Grammar(e.to_string()))?;
    let tree = parser.parse(source, None).ok_or(ParseError::Failed)?;
    let root = tree.root_node();
    if root.has_error() {
        let line = first_error(root).map_or(1, |n| n.start_position().row + 1);
        return Err(ParseError::Syntax { line });
    }

    let bytes = source.as_bytes();
    let mut out = ArenaTree::new(root.kind(), leaf_value(root, bytes), span_of(root));
    let root_id = out.root();
    convert_children(root, bytes, &mut out, root_id);
    Ok(out)
}

fn convert_children(node: tree_sitter::Node, src: &[u8], out: &mut ArenaTree, parent: NodeId) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let id = out.add_child(parent, child.kind(), leaf_value(child, src), span_of(child));
        if !child.is_named() {
            out.mark_anonymous(id);
        }
        convert_children(child, src, out, id);
    }
}

fn first_error(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
    if node.is_error() || node.is_missing() {
        return Some(node);
    }
    let mut cursor = node.walk();
    let failing = node.children(&mut cursor).find(|c| c.has_error());
    failing.and_then(first_error)
}

/// The text a node carries as its value: leaves always do. An inner node does too when
/// its children leave some of its text uncovered, as a comment does with its `//` token.
fn leaf_value(node: tree_sitter::Node, src: &[u8]) -> Option<String> {
    if node.child_count() == 0 || has_hidden_text(node, src) {
        node.utf8_text(src).ok().map(str::to_owned)
    } else {
        None
    }
}

/// True when a non-whitespace byte of `node` lies outside all of its children.
fn has_hidden_text(node: tree_sitter::Node, src: &[u8]) -> bool {
    let visible =
        |range: std::ops::Range<usize>| src[range].iter().any(|b| !b.is_ascii_whitespace());
    let mut pos = node.start_byte();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if visible(pos..child.start_byte()) {
            return true;
        }
        pos = child.end_byte();
    }
    visible(pos..node.end_byte())
}

fn span_of(node: tree_sitter::Node) -> Span {
    Span {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
    }
}
