//! GumTree-style node matching (Falleri et al., ASE 2014), the default engine.
//!
//! Matching runs in four steps:
//!
//! 1. Top-down, strict: identical subtrees (same structure, kinds and values) that have
//!    exactly one counterpart are matched, largest first.
//! 2. Containers: an unmatched inner node is matched to the unmatched node of the same
//!    kind with the highest Dice similarity over already-matched descendants, provided
//!    that similarity reaches [`MIN_DICE`].
//! 3. Top-down, loose: identical subtrees that still have several counterparts are
//!    matched, preferring the counterpart whose parent matches the old parent. Running
//!    this after step 2 is what makes the parent preference effective.
//! 4. Recovery: unmatched nodes whose parents are matched are matched to an unmatched
//!    child of the same kind under that parent, preferring equal values.
//!
//! The edit script is then derived from the mapping by [`crate::ops::script_from_mapping`],
//! which emits Chawathe-style insert, delete, move and update operations.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use crate::mapping::Mapping;
use crate::tree::{NodeId, Tree};

/// Minimum Dice similarity for matching inner nodes (GumTree default).
pub const MIN_DICE: f64 = 0.5;

/// Lower Dice threshold, used only when the parents of both nodes already correspond.
/// Renamed code keeps its structure but loses similarity, so it needs the relaxation.
pub const RELAXED_DICE: f64 = 0.3;

/// Matches the nodes of `old` and `new`.
pub fn mapping<A: Tree, B: Tree>(old: &A, new: &B) -> Mapping {
    let mut map = Mapping::new(old.node_count(), new.node_count());
    let old_hashes = subtree_hashes(old);
    let new_hashes = subtree_hashes(new);
    top_down(old, new, &old_hashes, &new_hashes, true, &mut map);
    containers(old, new, &mut map, MIN_DICE, false);
    top_down(old, new, &old_hashes, &new_hashes, false, &mut map);
    containers(old, new, &mut map, RELAXED_DICE, true);
    recover(old, new, &mut map);
    map
}

/// Structural hash of every subtree: equal hashes mean identical subtrees.
fn subtree_hashes<T: Tree>(tree: &T) -> Vec<u64> {
    let mut hashes = vec![0u64; tree.node_count()];
    for node in tree.postorder() {
        let mut hasher = DefaultHasher::new();
        tree.kind(node).hash(&mut hasher);
        tree.value(node).hash(&mut hasher);
        for &child in tree.children(node) {
            hashes[child].hash(&mut hasher);
        }
        hashes[node] = hasher.finish();
    }
    hashes
}

/// Matches identical subtrees, largest first. In `strict` mode only subtrees with a
/// single free counterpart are matched.
fn top_down<A: Tree, B: Tree>(
    old: &A,
    new: &B,
    old_hashes: &[u64],
    new_hashes: &[u64],
    strict: bool,
    map: &mut Mapping,
) {
    let mut candidates: HashMap<u64, Vec<NodeId>> = HashMap::new();
    for node in new.preorder() {
        candidates.entry(new_hashes[node]).or_default().push(node);
    }

    let heights = old.heights();
    let mut order = old.preorder();
    order.sort_by_key(|&node| std::cmp::Reverse(heights[node]));

    for node in order {
        if map.new_for(node).is_some() {
            continue;
        }
        let Some(pool) = candidates.get(&old_hashes[node]) else {
            continue;
        };
        let free: Vec<NodeId> = pool
            .iter()
            .copied()
            .filter(|&y| map.old_for(y).is_none())
            .filter(|&y| subtree_is_free(new, map, y))
            .collect();
        let parent_match = old.parent(node).and_then(|p| map.new_for(p));
        // Strict: only an unambiguous subtree. Loose: only a subtree whose parent
        // corresponds. Anything else is left for later steps that know more context.
        let chosen = if strict {
            (free.len() == 1).then(|| free[0])
        } else {
            free.iter()
                .copied()
                .find(|&y| parent_match.is_some() && new.parent(y) == parent_match)
        };
        if let Some(y) = chosen {
            for (x, z) in old.subtree(node).into_iter().zip(new.subtree(y)) {
                map.link(x, z);
            }
        }
    }
}

/// True when no node in the subtree of `node` is already matched.
fn subtree_is_free<B: Tree>(new: &B, map: &Mapping, node: NodeId) -> bool {
    new.subtree(node).iter().all(|&n| map.old_for(n).is_none())
}

/// Matches inner nodes by Dice similarity over matched descendants, children before
/// parents. With `require_parent`, a candidate must sit under the counterpart of x's parent.
fn containers<A: Tree, B: Tree>(
    old: &A,
    new: &B,
    map: &mut Mapping,
    threshold: f64,
    require_parent: bool,
) {
    if map.new_for(old.root()).is_none()
        && map.old_for(new.root()).is_none()
        && old.kind(old.root()) == new.kind(new.root())
    {
        map.link(old.root(), new.root());
    }

    for x in old.postorder() {
        if map.new_for(x).is_some() || old.is_leaf(x) {
            continue;
        }
        let parent_match = old.parent(x).and_then(|p| map.new_for(p));
        let old_desc = old.descendants(x);
        let mapped: Vec<NodeId> = old_desc.iter().filter_map(|&d| map.new_for(d)).collect();
        if mapped.is_empty() {
            continue;
        }

        // Only ancestors of the images of mapped descendants can share descendants with x.
        let mut candidates = HashSet::new();
        for &m in &mapped {
            let mut cur = new.parent(m);
            while let Some(p) = cur {
                if map.old_for(p).is_none() && old.kind(x) == new.kind(p) {
                    candidates.insert(p);
                }
                cur = new.parent(p);
            }
        }

        let mut best: Option<(f64, NodeId)> = None;
        for y in candidates {
            if require_parent && new.parent(y) != parent_match {
                continue;
            }
            let new_desc: HashSet<NodeId> = new.descendants(y).into_iter().collect();
            let common = mapped.iter().filter(|m| new_desc.contains(m)).count();
            let dice = 2.0 * common as f64 / (old_desc.len() + new_desc.len()) as f64;
            let better = match best {
                None => true,
                Some((score, id)) => dice > score || (dice == score && y < id),
            };
            if dice >= threshold && better {
                best = Some((dice, y));
            }
        }
        if let Some((_, y)) = best {
            map.link(x, y);
        }
    }
}

/// Matches unmatched old nodes to unmatched children of their matched parent's
/// counterpart. Pre-order, so a parent is recovered before its children.
fn recover<A: Tree, B: Tree>(old: &A, new: &B, map: &mut Mapping) {
    for x in old.preorder() {
        if map.new_for(x).is_some() {
            continue;
        }
        let Some(parent_new) = old.parent(x).and_then(|p| map.new_for(p)) else {
            continue;
        };
        let same_kind: Vec<NodeId> = new
            .children(parent_new)
            .iter()
            .copied()
            .filter(|&y| map.old_for(y).is_none() && new.kind(y) == old.kind(x))
            .collect();
        let pick = same_kind
            .iter()
            .copied()
            .find(|&y| new.value(y) == old.value(x))
            .or_else(|| same_kind.first().copied());
        if let Some(y) = pick {
            map.link(x, y);
        }
    }
}
