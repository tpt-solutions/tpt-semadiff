//! One-to-one correspondence between the nodes of an old and a new tree.

use crate::tree::NodeId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mapping {
    new_by_old: Vec<Option<NodeId>>,
    old_by_new: Vec<Option<NodeId>>,
}

impl Mapping {
    pub fn new(old_len: usize, new_len: usize) -> Self {
        Self {
            new_by_old: vec![None; old_len],
            old_by_new: vec![None; new_len],
        }
    }

    /// Links an old node with a new node. Returns `false` and changes nothing if
    /// either node is already matched.
    pub fn link(&mut self, old: NodeId, new: NodeId) -> bool {
        if self.new_by_old[old].is_some() || self.old_by_new[new].is_some() {
            return false;
        }
        self.new_by_old[old] = Some(new);
        self.old_by_new[new] = Some(old);
        true
    }

    /// The new-tree node matched to `old`, if any.
    pub fn new_for(&self, old: NodeId) -> Option<NodeId> {
        self.new_by_old[old]
    }

    /// The old-tree node matched to `new`, if any.
    pub fn old_for(&self, new: NodeId) -> Option<NodeId> {
        self.old_by_new[new]
    }

    /// All `(old, new)` pairs, ordered by old id.
    pub fn pairs(&self) -> impl Iterator<Item = (NodeId, NodeId)> + '_ {
        self.new_by_old
            .iter()
            .enumerate()
            .filter_map(|(old, new)| new.map(|new| (old, new)))
    }

    pub fn len(&self) -> usize {
        self.pairs().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
