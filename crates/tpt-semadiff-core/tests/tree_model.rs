//! Phase 1: the generic tree model, mappings and the edit-script derivation,
//! exercised on synthetic trees with no grammar involved.

use tpt_semadiff_core::ir::ArenaTree;
use tpt_semadiff_core::mapping::Mapping;
use tpt_semadiff_core::ops::{script_from_mapping, EditOp};
use tpt_semadiff_core::tree::Tree;

/// root(X(a, b), Y(c))
fn sample() -> ArenaTree {
    let mut t = ArenaTree::rooted("root");
    let x = t.add(0, "X", None);
    t.add(x, "leaf", Some("a"));
    t.add(x, "leaf", Some("b"));
    let y = t.add(0, "Y", None);
    t.add(y, "leaf", Some("c"));
    t
}

#[test]
fn postorder_visits_children_before_parents() {
    let t = sample();
    let post: Vec<_> = t
        .postorder()
        .iter()
        .map(|&n| t.kind(n).to_owned())
        .collect();
    assert_eq!(post.last().map(String::as_str), Some("root"));
    assert_eq!(post.len(), t.node_count());
    let order = t.postorder();
    for &n in &order {
        for &c in t.children(n) {
            let pc = order.iter().position(|&x| x == c).unwrap();
            let pn = order.iter().position(|&x| x == n).unwrap();
            assert!(pc < pn);
        }
    }
}

#[test]
fn preorder_and_subtree_start_at_the_given_node() {
    let t = sample();
    assert_eq!(t.preorder()[0], t.root());
    let x = t.children(t.root())[0];
    let sub = t.subtree(x);
    assert_eq!(sub[0], x);
    assert_eq!(sub.len(), 3);
    assert_eq!(t.descendants(x).len(), 2);
}

#[test]
fn heights_count_levels_with_leaves_at_one() {
    let t = sample();
    let h = t.heights();
    assert_eq!(h[t.root()], 3);
    assert_eq!(h[t.children(t.root())[0]], 2);
    assert_eq!(h[t.children(t.children(t.root())[0])[0]], 1);
}

#[test]
fn mapping_refuses_to_match_a_node_twice() {
    let mut m = Mapping::new(3, 3);
    assert!(m.link(0, 0));
    assert!(!m.link(0, 1), "old node 0 is already matched");
    assert!(!m.link(1, 0), "new node 0 is already matched");
    assert!(m.link(1, 1));
    assert_eq!(m.pairs().count(), 2);
    assert_eq!(m.new_for(0), Some(0));
    assert_eq!(m.old_for(1), Some(1));
}

#[test]
fn identical_trees_produce_no_operations() {
    let old = sample();
    let new = sample();
    let mut m = Mapping::new(old.node_count(), new.node_count());
    for (o, n) in old.preorder().into_iter().zip(new.preorder()) {
        m.link(o, n);
    }
    assert!(script_from_mapping(&old, &new, &m).is_empty());
}

#[test]
fn changed_value_is_an_update() {
    let old = sample();
    let mut new = ArenaTree::rooted("root");
    let x = new.add(0, "X", None);
    new.add(x, "leaf", Some("a"));
    new.add(x, "leaf", Some("b"));
    let y = new.add(0, "Y", None);
    new.add(y, "leaf", Some("z"));

    let mut m = Mapping::new(old.node_count(), new.node_count());
    for (o, n) in old.preorder().into_iter().zip(new.preorder()) {
        m.link(o, n);
    }
    let ops = script_from_mapping(&old, &new, &m);
    assert_eq!(ops.len(), 1);
    match &ops[0] {
        EditOp::Update {
            old_value,
            new_value,
            ..
        } => {
            assert_eq!(old_value.as_deref(), Some("c"));
            assert_eq!(new_value.as_deref(), Some("z"));
        }
        other => panic!("expected an update, got {other:?}"),
    }
}

#[test]
fn unmatched_subtree_is_reported_once_as_insert_and_delete() {
    let old = sample(); // root(X(a,b), Y(c))
    let mut new = ArenaTree::rooted("root");
    let x = new.add(0, "X", None);
    new.add(x, "leaf", Some("a"));
    new.add(x, "leaf", Some("b"));
    // Y is gone; Z(p, q) appears instead.
    let z = new.add(0, "Z", None);
    new.add(z, "leaf", Some("p"));
    new.add(z, "leaf", Some("q"));

    let mut m = Mapping::new(old.node_count(), new.node_count());
    m.link(0, 0);
    let x_old = old.children(0)[0];
    for (o, n) in old.subtree(x_old).into_iter().zip(new.subtree(x)) {
        m.link(o, n);
    }

    let ops = script_from_mapping(&old, &new, &m);
    let inserts = ops
        .iter()
        .filter(|op| matches!(op, EditOp::Insert { .. }))
        .count();
    let deletes = ops
        .iter()
        .filter(|op| matches!(op, EditOp::Delete { .. }))
        .count();
    // Z's subtree is one maximal insert; Y's subtree is one maximal delete.
    assert_eq!(inserts, 1);
    assert_eq!(deletes, 1);
}

#[test]
fn child_moved_under_another_parent_is_a_move() {
    let old = sample(); // root(X(a,b), Y(c))
    let mut new = ArenaTree::rooted("root");
    let x = new.add(0, "X", None);
    new.add(x, "leaf", Some("a"));
    let y = new.add(0, "Y", None);
    new.add(y, "leaf", Some("c"));
    new.add(y, "leaf", Some("b"));

    // old: X[a, b], Y[c]   new: X[a], Y[c, b]
    let mut m = Mapping::new(old.node_count(), new.node_count());
    m.link(0, 0);
    let (x_old, y_old) = (old.children(0)[0], old.children(0)[1]);
    m.link(x_old, x);
    m.link(y_old, y);
    m.link(old.children(x_old)[0], new.children(x)[0]); // a
    m.link(old.children(y_old)[0], new.children(y)[0]); // c
    m.link(old.children(x_old)[1], new.children(y)[1]); // b, moved under Y

    let ops = script_from_mapping(&old, &new, &m);
    assert_eq!(ops.len(), 1, "only the move should be reported: {ops:?}");
    assert!(matches!(ops[0], EditOp::Move { .. }));
}
