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
    pub fn cst(&self) -> &ArenaTree {
        &self.cst
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// The concrete-tree node behind a normalized node.
    pub fn origin(&self, node: NodeId) -> NodeId {
        self.origin[node]
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Source text covered by a node of the normalized tree.
    pub fn text(&self, node: NodeId) -> &str {
        let span = self.tree.span(node);
        &self.source[span.start_byte..span.end_byte]
    }

    /// Rebuilds the original source from the segments. Always equals the input.
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
    // `in_trivia[n]`: node `n` or an ancestor is trivia, so its leaves become trivia segments.
    let mut drop = vec![false; count];
    let mut in_trivia = vec![false; count];
    for node in cst.preorder() {
        let own = is_comment(cst.kind(node)) || (strip && is_docstring(&cst, node, lang));
        drop[node] = strip && own;
        in_trivia[node] = own || cst.parent(node).is_some_and(|p| in_trivia[p]);
    }

    let root = cst.root();
    let mut tree = ArenaTree::new(
        cst.kind(root),
        cst.value(root).map(str::to_owned),
        cst.span(root),
    );
    let mut origin = vec![root];
    copy_children(&cst, root, &mut tree, 0, &mut origin, &drop);

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
) {
    for &child in cst.children(node) {
        if drop[child] {
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
        copy_children(cst, child, out, id, origin, drop);
    }
}

fn is_comment(kind: &str) -> bool {
    kind.contains("comment")
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
