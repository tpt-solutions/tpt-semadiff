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
    TypeScript,
    /// TypeScript with JSX, which needs its own grammar.
    Tsx,
    Go,
}

impl SourceLang {
    /// Every language, in the order the docs list them.
    pub const ALL: [SourceLang; 6] = [
        Self::Rust,
        Self::Python,
        Self::JavaScript,
        Self::TypeScript,
        Self::Tsx,
        Self::Go,
    ];

    /// Picks a language from a file extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "rs" => Some(Self::Rust),
            "py" => Some(Self::Python),
            "js" | "mjs" | "cjs" | "jsx" => Some(Self::JavaScript),
            "ts" | "mts" | "cts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            "go" => Some(Self::Go),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Tsx => "tsx",
            Self::Go => "go",
        }
    }

    /// Picks a language from its [`name`](Self::name), ignoring case.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|lang| lang.name().eq_ignore_ascii_case(name.trim()))
    }

    /// Picks a language from a `#!` line, such as `#!/usr/bin/env python3`.
    pub fn from_shebang(line: &str) -> Option<Self> {
        let rest = line.strip_prefix("#!")?;
        let mut words = rest
            .split_whitespace()
            .map(|word| word.rsplit('/').next().unwrap_or(word));
        let first = words.next()?;
        // Through `env`, the interpreter follows any options, such as `-S`, and assignments.
        let interpreter = if first == "env" {
            words.find(|w| !w.starts_with('-') && !w.contains('='))?
        } else {
            first
        };
        let name = interpreter.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
        match name {
            "python" => Some(Self::Python),
            "node" | "nodejs" => Some(Self::JavaScript),
            _ => None,
        }
    }

    pub(crate) fn grammar(self) -> tree_sitter::Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
        }
    }

    /// Node kinds that introduce a function: named functions and methods, plus anonymous
    /// ones such as closures, lambdas and arrow functions.
    pub fn function_kinds(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["function_item", "closure_expression"],
            Self::Python => &["function_definition", "lambda"],
            Self::JavaScript | Self::TypeScript | Self::Tsx => &[
                "function_declaration",
                "generator_function_declaration",
                "method_definition",
                "function_expression",
                "arrow_function",
            ],
            Self::Go => &["function_declaration", "method_declaration", "func_literal"],
        }
    }

    /// Node kinds of a function's body block, whose children are its statements.
    pub fn body_kinds(self) -> &'static [&'static str] {
        match self {
            Self::Rust | Self::Python | Self::Go => &["block"],
            Self::JavaScript | Self::TypeScript | Self::Tsx => &["statement_block"],
        }
    }

    /// Node kinds that represent an import.
    pub fn import_kinds(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["use_declaration"],
            Self::Python => &["import_statement", "import_from_statement"],
            Self::JavaScript | Self::TypeScript | Self::Tsx => &["import_statement"],
            Self::Go => &["import_declaration", "import_spec"],
        }
    }

    /// Node kinds that bind a variable, for the inline-variable refactoring rule.
    pub fn binding_kinds(self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["let_declaration"],
            Self::Python => &["assignment"],
            Self::JavaScript | Self::TypeScript | Self::Tsx => {
                &["lexical_declaration", "variable_declaration"]
            }
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

/// Identifier kinds that can name a function or the binding it is assigned to.
const NAME_KINDS: &[&str] = &[
    "identifier",
    "field_identifier",
    "property_identifier",
    "name",
];

/// Anonymous function kinds whose direct identifier children are parameters, not a name.
/// JavaScript's `x => x` puts its single parameter directly under the node.
const PARAMETER_ONLY_KINDS: &[&str] = &[
    "arrow_function",
    "closure_expression",
    "lambda",
    "func_literal",
];

/// Parents that bind their value to a name, such as `const f = <function>`.
const BINDING_PARENTS: &[&str] = &[
    "variable_declarator",
    "let_declaration",
    "short_var_declaration",
    "assignment",
    "assignment_expression",
    "pair",
];

/// The name of a function node: its own name, else the name it is bound to
/// (`const f = () => ...`, `f = lambda: ...`). `None` for a function with neither.
///
/// The binding lookup is a heuristic. It takes the last name before the function in the
/// binding's left-hand side, which is the property for `obj.f = ...`.
pub fn function_name<T: Tree>(tree: &T, node: NodeId) -> Option<&str> {
    if !PARAMETER_ONLY_KINDS.contains(&tree.kind(node)) {
        let own = tree
            .children(node)
            .iter()
            .find(|&&c| NAME_KINDS.contains(&tree.kind(c)));
        if let Some(&name) = own {
            return tree.value(name);
        }
    }
    let parent = tree.parent(node)?;
    if !BINDING_PARENTS.contains(&tree.kind(parent)) {
        return None;
    }
    let start = tree.span(node).start_byte;
    tree.subtree(parent)
        .into_iter()
        .filter(|&n| NAME_KINDS.contains(&tree.kind(n)) && tree.span(n).end_byte <= start)
        .last()
        .and_then(|n| tree.value(n))
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
