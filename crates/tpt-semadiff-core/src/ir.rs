//! Arena-backed tree: the owned IR that front ends lower their parse trees into.

use crate::tree::{NodeId, Span, Tree};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Node {
    kind: String,
    value: Option<String>,
    span: Span,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    named: bool,
}

/// An owned tree stored in a flat arena. The root always has id 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArenaTree {
    nodes: Vec<Node>,
}

impl ArenaTree {
    /// Creates a tree that contains only a root node.
    pub fn new(kind: impl Into<String>, value: Option<String>, span: Span) -> Self {
        Self {
            nodes: vec![Node {
                kind: kind.into(),
                value,
                span,
                parent: None,
                children: Vec::new(),
                named: true,
            }],
        }
    }

    /// Marks a node as an anonymous token (punctuation or keyword).
    pub fn mark_anonymous(&mut self, node: NodeId) {
        self.nodes[node].named = false;
    }

    /// Creates a tree with a bare root of the given kind and an empty span.
    pub fn rooted(kind: &str) -> Self {
        Self::new(kind, None, Span::default())
    }

    /// Appends a child under `parent` and returns its id.
    pub fn add_child(
        &mut self,
        parent: NodeId,
        kind: impl Into<String>,
        value: Option<String>,
        span: Span,
    ) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(Node {
            kind: kind.into(),
            value,
            span,
            parent: Some(parent),
            children: Vec::new(),
            named: true,
        });
        self.nodes[parent].children.push(id);
        id
    }

    /// Shorthand for building trees in tests: appends a child with an empty span.
    pub fn add(&mut self, parent: NodeId, kind: &str, value: Option<&str>) -> NodeId {
        self.add_child(parent, kind, value.map(str::to_owned), Span::default())
    }
}

impl Tree for ArenaTree {
    fn root(&self) -> NodeId {
        0
    }

    fn node_count(&self) -> usize {
        self.nodes.len()
    }

    fn kind(&self, node: NodeId) -> &str {
        &self.nodes[node].kind
    }

    fn value(&self, node: NodeId) -> Option<&str> {
        self.nodes[node].value.as_deref()
    }

    fn children(&self, node: NodeId) -> &[NodeId] {
        &self.nodes[node].children
    }

    fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.nodes[node].parent
    }

    fn span(&self, node: NodeId) -> Span {
        self.nodes[node].span
    }

    fn is_named(&self, node: NodeId) -> bool {
        self.nodes[node].named
    }
}
