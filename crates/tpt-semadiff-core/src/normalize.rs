//! Trivia handling. Turns a parsed file into the tree that gets diffed, while keeping
//! enough information to rebuild the original bytes.
//!
//! Whitespace never appears in the tree. Comments stay in the tree under
//! [`TriviaPolicy::Keep`] and are dropped under [`TriviaPolicy::Strip`]. Python
//! docstrings are treated as comments under `Strip`, since they are documentation.

use std::ops::Range;

use crate::ir::ArenaTree;
use crate::parse::{parse_tree, ParseError, SourceLang};
use crate::tree::{NodeId, Tree};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TriviaPolicy {
    /// Comments stay in the tree and are compared like any other node.
    #[default]
    Keep,
    /// Comments and Python docstrings are removed before diffing.
    Strip,
}

/// One piece of the original source, in order. Concatenating all segments
/// reproduces the file exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// Bytes outside the normalized tree: whitespace, and comments that are kept as trivia.
    Trivia(Range<usize>),
    /// A token of the concrete tree, identified by its id in [`Normalized::cst`].
    Token(NodeId),
}

/// A file prepared for diffing.
#[derive(Debug, Clone)]
pub struct Normalized {
    source: String,
    cst: ArenaTree,
    tree: ArenaTree,
    /// For each node of `tree`, the id of the same node in `cst`.
    origin: Vec<NodeId>,
    segments: Vec<Segment>,
}

impl Normalized {
    /// The tree that gets diffed.
    pub fn tree(&self) -> &ArenaTree {
        &self.tree
    }

    /// The full concrete tree, including trivia.
    ///
    /// Library API. The CLI does not read this; it is kept so that tools can map a
    /// normalized node back to the source, or rebuild the file byte for byte.
    pub fn cst(&self) -> &ArenaTree {
        &self.cst
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// The concrete-tree node behind a normalized node.
    ///
    /// Library API. The CLI does not read this; see [`Normalized::cst`].
    pub fn origin(&self, node: NodeId) -> NodeId {
        self.origin[node]
    }

    /// The source split into trivia and tokens, in order.
    ///
    /// Library API. The CLI does not read this; see [`Normalized::cst`].
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Source text covered by a node of the normalized tree.
    pub fn text(&self, node: NodeId) -> &str {
        let span = self.tree.span(node);
        &self.source[span.start_byte..span.end_byte]
    }

    /// Rebuilds the original source from the segments. Always equals the input.
    ///
    /// Library API. The CLI does not read this; see [`Normalized::cst`].
    pub fn reconstruct(&self) -> String {
        let mut out = String::with_capacity(self.source.len());
        for segment in &self.segments {
            match segment {
                Segment::Trivia(range) => out.push_str(&self.source[range.clone()]),
                Segment::Token(id) => {
                    let span = self.cst.span(*id);
                    out.push_str(&self.source[span.start_byte..span.end_byte]);
                }
            }
        }
        out
    }
}

/// Parses and normalizes `source`.
pub fn normalize(
    source: &str,
    lang: SourceLang,
    policy: TriviaPolicy,
) -> Result<Normalized, ParseError> {
    let cst = parse_tree(source, lang)?;
    let strip = policy == TriviaPolicy::Strip;
    let count = cst.node_count();

    // `drop[n]`: node `n` is removed from the diffed tree.
    // `unwrap[n]`: node `n` is removed, but its children move up to its parent.
    // `in_trivia[n]`: node `n` or an ancestor is trivia, so its leaves become trivia segments.
    let mut drop = vec![false; count];
    let mut unwrap = vec![false; count];
    let mut in_trivia = vec![false; count];
    for node in cst.preorder() {
        let own = is_comment(cst.kind(node))
            || (strip
                && (is_docstring(&cst, node, lang)
                    || is_trailing_comma(&cst, node)
                    || is_redundant_paren(&cst, node)));
        drop[node] = strip && own;
        unwrap[node] = strip && is_paren_wrapper(&cst, node);
        in_trivia[node] = own || cst.parent(node).is_some_and(|p| in_trivia[p]);
    }

    let root = cst.root();
    let mut tree = ArenaTree::new(
        cst.kind(root),
        cst.value(root).map(str::to_owned),
        cst.span(root),
    );
    let mut origin = vec![root];
    copy_children(&cst, root, &mut tree, 0, &mut origin, &drop, &unwrap);

    let mut leaves: Vec<NodeId> = cst
        .preorder()
        .into_iter()
        .filter(|&n| cst.is_leaf(n) && cst.span(n).end_byte > cst.span(n).start_byte)
        .collect();
    leaves.sort_by_key(|&n| cst.span(n).start_byte);

    let mut segments = Vec::new();
    let mut pos = 0;
    for leaf in leaves {
        let span = cst.span(leaf);
        if span.start_byte > pos {
            segments.push(Segment::Trivia(pos..span.start_byte));
        }
        if in_trivia[leaf] {
            segments.push(Segment::Trivia(span.start_byte..span.end_byte));
        } else {
            segments.push(Segment::Token(leaf));
        }
        pos = pos.max(span.end_byte);
    }
    if pos < source.len() {
        segments.push(Segment::Trivia(pos..source.len()));
    }

    Ok(Normalized {
        source: source.to_owned(),
        cst,
        tree,
        origin,
        segments,
    })
}

fn copy_children(
    cst: &ArenaTree,
    node: NodeId,
    out: &mut ArenaTree,
    out_parent: NodeId,
    origin: &mut Vec<NodeId>,
    drop: &[bool],
    unwrap: &[bool],
) {
    for &child in cst.children(node) {
        if drop[child] {
            continue;
        }
        if unwrap[child] {
            copy_children(cst, child, out, out_parent, origin, drop, unwrap);
            continue;
        }
        let id = out.add_child(
            out_parent,
            cst.kind(child),
            cst.value(child).map(str::to_owned),
            cst.span(child),
        );
        if !cst.is_named(child) {
            out.mark_anonymous(id);
        }
        origin.push(child);
        copy_children(cst, child, out, id, origin, drop, unwrap);
    }
}

fn is_comment(kind: &str) -> bool {
    kind.contains("comment")
}

/// Node kinds whose children are a list of items separated by commas. A comma after the
/// last item only changes the source text, so `--ignore-formatting` drops it there.
/// Tuples are left out: in Python `(1,)` is a tuple and its comma is significant.
const LIST_KINDS: &[&str] = &[
    "arguments",
    "argument_list",
    "parameters",
    "formal_parameters",
    "parameter_list",
    "lambda_parameters",
    "array",
    "array_expression",
    "object",
    "list",
    "dictionary",
    "set",
    "array_pattern",
    "object_pattern",
    "initializer_list",
    "field_initializer_list",
    "field_declaration_list",
    "declaration_list",
    "enum_variant_list",
    "literal_value",
];

/// An anonymous comma that ends a list: it follows an item and comes right before a closing
/// bracket. A comma directly after another comma is kept, since it makes an array hole.
fn is_trailing_comma(cst: &ArenaTree, node: NodeId) -> bool {
    if cst.is_named(node) || cst.kind(node) != "," {
        return false;
    }
    let Some(parent) = cst.parent(node) else {
        return false;
    };
    if !LIST_KINDS.contains(&cst.kind(parent)) {
        return false;
    }
    let siblings = cst.children(parent);
    let Some(pos) = siblings.iter().position(|&c| c == node) else {
        return false;
    };
    let follows_item = pos > 0 && cst.is_named(siblings[pos - 1]);
    let closes = siblings
        .get(pos + 1)
        .is_some_and(|&next| !cst.is_named(next) && matches!(cst.kind(next), ")" | "]" | "}"));
    follows_item && closes
}

/// A parenthesized expression, whose parentheses carry no meaning of their own. The tree
/// keeps the expression inside, so `(a + b) * c` still differs from `a + b * c`.
fn is_paren_wrapper(cst: &ArenaTree, node: NodeId) -> bool {
    cst.kind(node) == "parenthesized_expression" && cst.parent(node).is_some()
}

/// The `(` or `)` token of a parenthesized expression that is being unwrapped.
fn is_redundant_paren(cst: &ArenaTree, node: NodeId) -> bool {
    !cst.is_named(node)
        && matches!(cst.kind(node), "(" | ")")
        && cst.parent(node).is_some_and(|p| is_paren_wrapper(cst, p))
}

/// A Python docstring: the first statement of a module or block, if it is a bare string.
fn is_docstring(cst: &ArenaTree, node: NodeId, lang: SourceLang) -> bool {
    if lang != SourceLang::Python || cst.kind(node) != "expression_statement" {
        return false;
    }
    let &[only] = cst.children(node) else {
        return false;
    };
    if cst.kind(only) != "string" {
        return false;
    }
    let Some(parent) = cst.parent(node) else {
        return false;
    };
    matches!(cst.kind(parent), "module" | "block")
        && cst
            .children(parent)
            .iter()
            .find(|&&c| !is_comment(cst.kind(c)))
            == Some(&node)
}
