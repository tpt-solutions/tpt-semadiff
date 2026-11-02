//! Semantic edit operations, and the script that derives them from a mapping.

use std::collections::HashSet;

use crate::mapping::Mapping;
use crate::tree::{NodeId, Tree};

/// One semantic change.
///
/// Node ids refer to the old tree for `Delete`, to the new tree for `Insert`,
/// and to both trees for `Move` and `Update`. `Insert` and `Delete` are reported
/// once per maximal unmatched subtree, not once per node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditOp {
    Insert {
        node: NodeId,
        parent: Option<NodeId>,
        index: usize,
        kind: String,
        value: Option<String>,
    },
    Delete {
        node: NodeId,
        kind: String,
        value: Option<String>,
    },
    /// A matched node that moved to another parent or changed its position among siblings.
    Move {
        old: NodeId,
        new: NodeId,
        parent: NodeId,
        index: usize,
        kind: String,
    },
    /// A matched node whose kind or value changed.
    Update {
        old: NodeId,
        new: NodeId,
        old_kind: String,
        new_kind: String,
        old_value: Option<String>,
        new_value: Option<String>,
    },
}

/// Derives the edit script that turns `old` into `new` under `mapping`.
pub fn script_from_mapping<A: Tree, B: Tree>(old: &A, new: &B, mapping: &Mapping) -> Vec<EditOp> {
    let mut ops = Vec::new();

    for node in new.preorder() {
        if let Some(o) = mapping.old_for(node) {
            if old.kind(o) != new.kind(node) || old.value(o) != new.value(node) {
                ops.push(EditOp::Update {
                    old: o,
                    new: node,
                    old_kind: old.kind(o).to_owned(),
                    new_kind: new.kind(node).to_owned(),
                    old_value: old.value(o).map(str::to_owned),
                    new_value: new.value(node).map(str::to_owned),
                });
            }
        }
    }

    if mapping.old_for(new.root()).is_none() {
        ops.push(insert(new, new.root(), None, 0));
    }

    for parent in new.preorder() {
        let stationary = stationary_children(old, new, mapping, parent);
        for (index, &child) in new.children(parent).iter().enumerate() {
            match mapping.old_for(child) {
                // Children of an unmatched parent are covered by that parent's insert.
                None if mapping.old_for(parent).is_some() => {
                    ops.push(insert(new, child, Some(parent), index));
                }
                None => {}
                Some(o) if !stationary.contains(&o) => ops.push(EditOp::Move {
                    old: o,
                    new: child,
                    parent,
                    index,
                    kind: new.kind(child).to_owned(),
                }),
                Some(_) => {}
            }
        }
    }

    for node in old.preorder() {
        if mapping.new_for(node).is_some() {
            continue;
        }
        let covered = old
            .parent(node)
            .is_some_and(|p| mapping.new_for(p).is_none());
        if !covered {
            ops.push(EditOp::Delete {
                node,
                kind: old.kind(node).to_owned(),
                value: old.value(node).map(str::to_owned),
            });
        }
    }

    ops
}

fn insert<B: Tree>(new: &B, node: NodeId, parent: Option<NodeId>, index: usize) -> EditOp {
    EditOp::Insert {
        node,
        parent,
        index,
        kind: new.kind(node).to_owned(),
        value: new.value(node).map(str::to_owned),
    }
}

/// Old ids of the children of `parent` (a new-tree node) that keep their place: they are
/// matched to children of the old counterpart of `parent`, and they lie on a longest
/// common subsequence of the two sibling orders. Every other matched child gets a `Move`.
fn stationary_children<A: Tree, B: Tree>(
    old: &A,
    new: &B,
    mapping: &Mapping,
    parent: NodeId,
) -> HashSet<NodeId> {
    let Some(old_parent) = mapping.old_for(parent) else {
        return HashSet::new();
    };
    let in_new_order: Vec<NodeId> = new
        .children(parent)
        .iter()
        .filter_map(|&c| mapping.old_for(c))
        .filter(|&o| old.parent(o) == Some(old_parent))
        .collect();
    let in_old_order: Vec<NodeId> = old
        .children(old_parent)
        .iter()
        .copied()
        .filter(|&o| {
            mapping
                .new_for(o)
                .is_some_and(|c| new.parent(c) == Some(parent))
        })
        .collect();
    lcs_members(&in_new_order, &in_old_order)
}

/// Elements of `a` that belong to some longest common subsequence of `a` and `b`.
/// Elements of `a` must be distinct.
fn lcs_members(a: &[NodeId], b: &[NodeId]) -> HashSet<NodeId> {
    let (n, m) = (a.len(), b.len());
    let mut len = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            len[i][j] = if a[i] == b[j] {
                len[i + 1][j + 1] + 1
            } else {
                len[i + 1][j].max(len[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut members = HashSet::new();
    while i < n && j < m {
        if a[i] == b[j] {
            members.insert(a[i]);
            i += 1;
            j += 1;
        } else if len[i + 1][j] >= len[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    members
}
