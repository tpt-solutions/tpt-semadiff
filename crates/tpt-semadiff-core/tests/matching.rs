//! Phases 4 and 5: Zhang-Shasha distances and mappings, GumTree matching, and a
//! regression check that the two engines agree on simple, known edits.

use tpt_semadiff_core::diff::{diff_trees, Engine};
use tpt_semadiff_core::gumtree;
use tpt_semadiff_core::ir::ArenaTree;
use tpt_semadiff_core::ops::EditOp;
use tpt_semadiff_core::tree::Tree;
use tpt_semadiff_core::zhang_shasha;

/// f(a, b(c)) — a small labelled tree.
fn base() -> ArenaTree {
    let mut t = ArenaTree::rooted("f");
    t.add(0, "leaf", Some("a"));
    let b = t.add(0, "b", None);
    t.add(b, "leaf", Some("c"));
    t
}

fn sorted_kinds(ops: &[EditOp]) -> (usize, usize, usize, usize) {
    let mut counts = (0, 0, 0, 0);
    for op in ops {
        match op {
            EditOp::Insert { .. } => counts.0 += 1,
            EditOp::Delete { .. } => counts.1 += 1,
            EditOp::Move { .. } => counts.2 += 1,
            EditOp::Update { .. } => counts.3 += 1,
        }
    }
    counts
}

#[test]
fn zhang_shasha_distance_of_identical_trees_is_zero() {
    let (distance, map) = zhang_shasha::mapping(&base(), &base());
    assert_eq!(distance, 0);
    assert_eq!(map.pairs().count(), base().node_count());
}

#[test]
fn zhang_shasha_relabel_costs_one() {
    let mut new = ArenaTree::rooted("f");
    new.add(0, "leaf", Some("z"));
    let b = new.add(0, "b", None);
    new.add(b, "leaf", Some("c"));
    let (distance, _) = zhang_shasha::mapping(&base(), &new);
    assert_eq!(distance, 1);
}

#[test]
fn zhang_shasha_insert_and_delete_of_a_leaf_cost_one() {
    let mut bigger = base();
    bigger.add(0, "leaf", Some("extra"));
    assert_eq!(zhang_shasha::mapping(&base(), &bigger).0, 1);
    assert_eq!(zhang_shasha::mapping(&bigger, &base()).0, 1);
}

#[test]
fn zhang_shasha_inserting_a_two_node_subtree_costs_two() {
    let mut bigger = base();
    let n = bigger.add(0, "g", None);
    bigger.add(n, "leaf", Some("h"));
    assert_eq!(zhang_shasha::mapping(&base(), &bigger).0, 2);
}

#[test]
fn zhang_shasha_swapping_siblings_costs_two_without_moves() {
    let mut ab = ArenaTree::rooted("r");
    ab.add(0, "leaf", Some("a"));
    ab.add(0, "leaf", Some("b"));
    let mut ba = ArenaTree::rooted("r");
    ba.add(0, "leaf", Some("b"));
    ba.add(0, "leaf", Some("a"));
    assert_eq!(zhang_shasha::mapping(&ab, &ba).0, 2);
}

#[test]
fn zhang_shasha_mapping_preserves_postorder() {
    let old = base();
    let mut new = base();
    new.add(0, "leaf", Some("extra"));
    let (_, map) = zhang_shasha::mapping(&old, &new);
    let old_post = old.postorder();
    let new_post = new.postorder();
    let pairs: Vec<_> = map.pairs().collect();
    for &(o1, n1) in &pairs {
        for &(o2, n2) in &pairs {
            let old_before =
                old_post.iter().position(|&x| x == o1) < old_post.iter().position(|&x| x == o2);
            let new_before =
                new_post.iter().position(|&x| x == n1) < new_post.iter().position(|&x| x == n2);
            assert_eq!(old_before, new_before, "mapping must preserve postorder");
        }
    }
}

#[test]
fn gumtree_identical_trees_have_no_operations() {
    let (_, ops) = diff_trees(&base(), &base(), Engine::Chawathe);
    assert!(ops.is_empty());
}

#[test]
fn gumtree_detects_a_move_across_subtrees() {
    // old: X[a, b], Y[c]      new: X[a], Y[c, b]
    let mut old = ArenaTree::rooted("root");
    let x = old.add(0, "X", None);
    old.add(x, "leaf", Some("a"));
    old.add(x, "leaf", Some("bee"));
    let y = old.add(0, "Y", None);
    old.add(y, "leaf", Some("c"));
    old.add(y, "leaf", Some("d"));

    let mut new = ArenaTree::rooted("root");
    let x2 = new.add(0, "X", None);
    new.add(x2, "leaf", Some("a"));
    let y2 = new.add(0, "Y", None);
    new.add(y2, "leaf", Some("c"));
    new.add(y2, "leaf", Some("d"));
    new.add(y2, "leaf", Some("bee"));

    let (_, ops) = diff_trees(&old, &new, Engine::Chawathe);
    assert_eq!(sorted_kinds(&ops), (0, 0, 1, 0), "{ops:?}");
}

#[test]
fn gumtree_matches_a_renamed_leaf_as_an_update() {
    let mut old = ArenaTree::rooted("fn");
    let body = old.add(0, "body", None);
    old.add(body, "identifier", Some("sum"));
    old.add(body, "op", Some("+"));
    old.add(body, "identifier", Some("x"));

    let mut new = ArenaTree::rooted("fn");
    let body2 = new.add(0, "body", None);
    new.add(body2, "identifier", Some("acc"));
    new.add(body2, "op", Some("+"));
    new.add(body2, "identifier", Some("x"));

    let (_, ops) = diff_trees(&old, &new, Engine::Chawathe);
    assert_eq!(sorted_kinds(&ops), (0, 0, 0, 1), "{ops:?}");
}

#[test]
fn gumtree_mapping_pairs_have_matching_kinds_for_identical_leaves() {
    let old = base();
    let new = base();
    let map = gumtree::mapping(&old, &new);
    for (o, n) in map.pairs() {
        assert_eq!(old.kind(o), new.kind(n));
    }
}

#[test]
fn engines_agree_on_counts_for_simple_known_edits() {
    // Each case: (old, new) with a single known edit; both engines must report the same counts.
    let cases: Vec<(ArenaTree, ArenaTree)> = vec![
        (base(), base()),
        {
            let mut n = base();
            n.add(0, "leaf", Some("new"));
            (base(), n)
        },
        {
            let mut n = ArenaTree::rooted("f");
            n.add(0, "leaf", Some("a"));
            let b = n.add(0, "b", None);
            n.add(b, "leaf", Some("zz"));
            (base(), n)
        },
    ];
    for (old, new) in cases {
        let (_, chawathe) = diff_trees(&old, &new, Engine::Chawathe);
        let (_, zs) = diff_trees(&old, &new, Engine::ZhangShasha);
        assert_eq!(
            sorted_kinds(&chawathe),
            sorted_kinds(&zs),
            "engines disagree: chawathe={chawathe:?} zs={zs:?}"
        );
    }
}
