//! Language-agnostic tree interface shared by every diff algorithm.

/// Index of a node inside a tree. Ids are dense: `0..node_count()`.
pub type NodeId = usize;

/// Location of a node in its source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start_byte: usize,
    pub end_byte: usize,
    /// 1-based line of the first byte.
    pub start_line: usize,
    /// 1-based line of the last byte.
    pub end_line: usize,
}

/// An ordered, rooted tree whose nodes have a kind, an optional value and a span.
///
/// Algorithms only see this trait, so they stay independent of any parser.
/// Traversals are provided on top of the required accessors.
pub trait Tree {
    fn root(&self) -> NodeId;
    fn node_count(&self) -> usize;
    fn kind(&self, node: NodeId) -> &str;
    fn value(&self, node: NodeId) -> Option<&str>;
    fn children(&self, node: NodeId) -> &[NodeId];
    fn parent(&self, node: NodeId) -> Option<NodeId>;
    fn span(&self, node: NodeId) -> Span;

    /// False for anonymous tokens such as punctuation and keywords. Defaults to true.
    fn is_named(&self, _node: NodeId) -> bool {
        true
    }

    fn is_leaf(&self, node: NodeId) -> bool {
        self.children(node).is_empty()
    }

    /// Nodes of the subtree rooted at `node`, in pre-order (parent before children).
    fn subtree(&self, node: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            out.push(current);
            stack.extend(self.children(current).iter().rev().copied());
        }
        out
    }

    /// Every node in pre-order.
    fn preorder(&self) -> Vec<NodeId> {
        self.subtree(self.root())
    }

    /// Every node in post-order (children before parent).
    fn postorder(&self) -> Vec<NodeId> {
        let mut out = Vec::with_capacity(self.node_count());
        let mut stack = vec![(self.root(), false)];
        while let Some((node, expanded)) = stack.pop() {
            if expanded {
                out.push(node);
            } else {
                stack.push((node, true));
                stack.extend(self.children(node).iter().rev().map(|&c| (c, false)));
            }
        }
        out
    }

    /// Descendants of `node`, excluding `node` itself.
    fn descendants(&self, node: NodeId) -> Vec<NodeId> {
        let mut nodes = self.subtree(node);
        nodes.remove(0);
        nodes
    }

    /// Height of every node, indexed by id. Leaves have height 1.
    fn heights(&self) -> Vec<usize> {
        let mut heights = vec![0; self.node_count()];
        for node in self.postorder() {
            heights[node] = 1 + self
                .children(node)
                .iter()
                .map(|&c| heights[c])
                .max()
                .unwrap_or(0);
        }
        heights
    }
}
