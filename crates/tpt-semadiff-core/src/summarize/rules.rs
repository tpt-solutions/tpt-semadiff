//! Rule-based summarizer. Each rule matches a pattern in the semantic operations, or in
//! the statements of the functions, and phrases it as a sentence. Rules are independent;
//! one change can match several.

use std::collections::BTreeMap;

use crate::diff::FileDiff;
use crate::normalize::Normalized;
use crate::ops::EditOp;
use crate::parse::{function_name, SourceLang};
use crate::tree::{NodeId, Tree};

/// Summarizes a diff as sentences. Returns one line per recognized refactoring, or a
/// count of operations when none is recognized.
pub fn summarize(diff: &FileDiff) -> Vec<String> {
    if diff.ops.is_empty() {
        return vec!["No structural changes.".to_owned()];
    }
    let mut lines = Vec::new();
    lines.extend(rename_lines(diff));
    lines.extend(import_lines(diff));
    lines.extend(extract_lines(diff));
    lines.extend(inline_lines(diff));
    if lines.is_empty() {
        lines.push(count_line(diff));
    }
    lines
}

fn count_line(diff: &FileDiff) -> String {
    let (mut inserts, mut deletes, mut moves, mut updates) = (0, 0, 0, 0);
    for op in &diff.ops {
        match op {
            EditOp::Insert { .. } => inserts += 1,
            EditOp::Delete { .. } => deletes += 1,
            EditOp::Move { .. } => moves += 1,
            EditOp::Update { .. } => updates += 1,
        }
    }
    format!("{inserts} insertions, {deletes} deletions, {moves} moves, {updates} updates")
}

/// `Renamed a to b`: identifier leaves updated to new names.
fn rename_lines(diff: &FileDiff) -> Vec<String> {
    let new = diff.new.tree();
    let mut pairs: BTreeMap<(String, String), usize> = BTreeMap::new();
    for op in &diff.ops {
        if let EditOp::Update {
            new: node,
            old_value: Some(old),
            new_value: Some(name),
            ..
        } = op
        {
            if new.kind(*node).ends_with("identifier") {
                *pairs.entry((old.clone(), name.clone())).or_default() += 1;
            }
        }
    }
    pairs
        .into_iter()
        .map(|((old, name), count)| {
            if count > 1 {
                format!("Renamed `{old}` to `{name}` ({count} occurrences)")
            } else {
                format!("Renamed `{old}` to `{name}`")
            }
        })
        .collect()
}

/// `Reordered imports`: only imports changed, and some of them moved.
fn import_lines(diff: &FileDiff) -> Vec<String> {
    let moves = diff
        .ops
        .iter()
        .filter(|op| matches!(op, EditOp::Move { .. }))
        .count();
    let kinds = diff.lang.import_kinds();
    let all_imports = diff.ops.iter().all(|op| op_in_kinds(diff, op, kinds));
    if moves > 0 && all_imports {
        vec![format!("Reordered imports ({moves} moved)")]
    } else {
        Vec::new()
    }
}

fn op_in_kinds(diff: &FileDiff, op: &EditOp, kinds: &[&str]) -> bool {
    match op {
        EditOp::Insert { node, .. } => has_ancestor_kind(diff.new.tree(), *node, kinds),
        EditOp::Delete { node, .. } => has_ancestor_kind(diff.old.tree(), *node, kinds),
        EditOp::Move { new, .. } | EditOp::Update { new, .. } => {
            has_ancestor_kind(diff.new.tree(), *new, kinds)
        }
    }
}

fn has_ancestor_kind<T: Tree>(tree: &T, node: NodeId, kinds: &[&str]) -> bool {
    let mut cur = Some(node);
    while let Some(n) = cur {
        if kinds.contains(&tree.kind(n)) {
            return true;
        }
        cur = tree.parent(n);
    }
    false
}

/// `Extracted f from g`: a function that is new in the file and holds statements that
/// the old version had in another function. Matching by statement text keeps this rule
/// independent of how the tree matcher paired the nodes.
fn extract_lines(diff: &FileDiff) -> Vec<String> {
    let old_fns = functions(&diff.old, diff.lang);
    let new_fns = functions(&diff.new, diff.lang);
    let mut lines = Vec::new();
    for (name, statements) in &new_fns {
        if old_fns.iter().any(|(old_name, _)| old_name == name) || statements.is_empty() {
            continue;
        }
        let mut best: Option<(&str, usize)> = None;
        for (old_name, old_statements) in &old_fns {
            let shared = statements
                .iter()
                .filter(|s| old_statements.contains(s))
                .count();
            if shared > best.map_or(0, |(_, n)| n) {
                best = Some((old_name, shared));
            }
        }
        if let Some((source, shared)) = best {
            lines.push(format!(
                "Extracted `{name}` from `{source}` ({shared} statements moved into it)"
            ));
        }
    }
    lines
}

/// Name and body statements (whitespace-collapsed text) of every named function in a file.
fn functions(file: &Normalized, lang: SourceLang) -> Vec<(String, Vec<String>)> {
    let tree = file.tree();
    tree.preorder()
        .into_iter()
        .filter(|&n| lang.function_kinds().contains(&tree.kind(n)))
        .filter_map(|n| {
            Some((
                function_name(tree, n)?.to_owned(),
                body_statements(file, lang, n),
            ))
        })
        .collect()
}

fn body_statements(file: &Normalized, lang: SourceLang, function: NodeId) -> Vec<String> {
    let tree = file.tree();
    let Some(&body) = tree
        .children(function)
        .iter()
        .find(|&&c| lang.body_kinds().contains(&tree.kind(c)))
    else {
        return Vec::new();
    };
    tree.children(body)
        .iter()
        .filter(|&&c| tree.is_named(c))
        .map(|&c| collapse(file.text(c)))
        .collect()
}

/// `Inlined variable x`: a binding was removed and its initializer now appears verbatim,
/// while the name no longer occurs anywhere.
fn inline_lines(diff: &FileDiff) -> Vec<String> {
    let (old, new) = (diff.old.tree(), diff.new.tree());
    let bindings = diff.lang.binding_kinds();
    let new_source = collapse(diff.new.text(new.root()));
    let mut lines = Vec::new();
    for op in &diff.ops {
        let EditOp::Delete { node, .. } = op else {
            continue;
        };
        // Python wraps an assignment in an expression statement, so the deleted node can be
        // the wrapper. Look one level down for the binding.
        let binding = if bindings.contains(&old.kind(*node)) {
            *node
        } else {
            let Some(&inner) = old
                .children(*node)
                .iter()
                .find(|&&c| bindings.contains(&old.kind(c)))
            else {
                continue;
            };
            inner
        };
        let Some(name) = old
            .subtree(binding)
            .into_iter()
            .find(|&n| old.kind(n).ends_with("identifier"))
            .and_then(|n| old.value(n))
            .map(str::to_owned)
        else {
            continue;
        };
        let Some(value) = initializer(old, binding) else {
            continue;
        };
        let expr = collapse(diff.old.text(value));
        if expr.is_empty() {
            continue;
        }
        let still_bound = new
            .preorder()
            .into_iter()
            .any(|n| new.value(n) == Some(name.as_str()));
        if !still_bound && new_source.contains(&expr) {
            lines.push(format!("Inlined variable `{name}` (`{expr}`)"));
        }
    }
    lines
}

/// The expression a binding assigns: the last named child of the binding, or of its
/// declarator for `const x = ...`. `None` when there is no `=` or `:=` (a bare declaration).
fn initializer<T: Tree>(tree: &T, binding: NodeId) -> Option<NodeId> {
    let decl = tree
        .children(binding)
        .iter()
        .copied()
        .find(|&c| tree.kind(c) == "variable_declarator")
        .unwrap_or(binding);
    let assigns = tree
        .children(decl)
        .iter()
        .any(|&c| !tree.is_named(c) && matches!(tree.kind(c), "=" | ":="));
    if !assigns {
        return None;
    }
    tree.children(decl)
        .iter()
        .rev()
        .copied()
        .find(|&c| tree.is_named(c))
}

fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
