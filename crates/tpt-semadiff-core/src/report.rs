//! Semantic report: edit operations grouped by the function they fall in.
//! This is the structure the text, JSON and TUI front ends all render.

use serde::Serialize;

use crate::diff::FileDiff;
use crate::normalize::Normalized;
use crate::ops::EditOp;
use crate::parse::SourceLang;
use crate::tree::{NodeId, Tree};

/// Longest source snippet quoted in a description.
const SNIPPET_CHARS: usize = 60;

/// Names a function node can carry as a direct child.
const NAME_KINDS: &[&str] = &[
    "identifier",
    "field_identifier",
    "property_identifier",
    "name",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OpKind {
    Insert,
    Delete,
    Move,
    Update,
}

/// One rendered change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Change {
    pub op: OpKind,
    /// Kind of the affected node (the new kind for updates).
    pub node_kind: String,
    /// 1-based line in the old file, when the change has an old side.
    pub old_line: Option<usize>,
    /// 1-based line in the new file, when the change has a new side.
    pub new_line: Option<usize>,
    pub description: String,
}

/// Changes that fall inside one function, or at top level when `function` is `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Group {
    pub function: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub changes: Vec<Change>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ChangeCounts {
    pub insert: usize,
    pub delete: usize,
    #[serde(rename = "move")]
    pub moved: usize,
    pub update: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub old_name: String,
    pub new_name: String,
    pub language: &'static str,
    pub engine: &'static str,
    pub counts: ChangeCounts,
    pub groups: Vec<Group>,
    /// Optional natural-language summary lines, filled in by the caller.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub summaries: Vec<String>,
}

/// A change together with the function it belongs to.
struct Placed {
    change: Change,
    function: Option<(String, Option<(usize, usize)>)>,
}

impl Report {
    pub fn from_diff(
        diff: &FileDiff,
        old_name: &str,
        new_name: &str,
        engine: &'static str,
    ) -> Self {
        let mut report = Report {
            old_name: old_name.to_owned(),
            new_name: new_name.to_owned(),
            language: diff.lang.name(),
            engine,
            counts: ChangeCounts::default(),
            groups: Vec::new(),
            summaries: Vec::new(),
        };
        for op in &diff.ops {
            // A moved punctuation or keyword token is a side effect of its parent's
            // move, so it would only add noise.
            if let EditOp::Move { new, .. } = op {
                if !diff.new.tree().is_named(*new) {
                    continue;
                }
            }
            let placed = place(diff, op);
            match placed.change.op {
                OpKind::Insert => report.counts.insert += 1,
                OpKind::Delete => report.counts.delete += 1,
                OpKind::Move => report.counts.moved += 1,
                OpKind::Update => report.counts.update += 1,
            }
            add_to_group(&mut report.groups, placed);
        }
        for group in &mut report.groups {
            group.changes.sort_by_key(|c| c.new_line.or(c.old_line));
        }
        report.groups.sort_by_key(|g| g.start_line);
        report
    }

    /// Pretty-printed JSON, following the schema in `docs/json-schema.md`.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Names of the functions that contain at least one change.
    pub fn changed_functions(&self) -> impl Iterator<Item = &str> {
        self.groups.iter().filter_map(|g| g.function.as_deref())
    }
}

fn place(diff: &FileDiff, op: &EditOp) -> Placed {
    let (old, new) = (&diff.old, &diff.new);
    match op {
        EditOp::Insert { node, kind, .. } => {
            let line = new.tree().span(*node).start_line;
            Placed {
                function: enclosing_function(new, *node, diff.lang),
                change: Change {
                    op: OpKind::Insert,
                    node_kind: kind.clone(),
                    old_line: None,
                    new_line: Some(line),
                    description: format!("Added `{}`", snippet(new.text(*node))),
                },
            }
        }
        EditOp::Delete { node, kind, .. } => {
            let line = old.tree().span(*node).start_line;
            Placed {
                function: enclosing_function(old, *node, diff.lang),
                change: Change {
                    op: OpKind::Delete,
                    node_kind: kind.clone(),
                    old_line: Some(line),
                    new_line: None,
                    description: format!("Removed `{}`", snippet(old.text(*node))),
                },
            }
        }
        EditOp::Move {
            old: o,
            new: n,
            kind,
            ..
        } => {
            let new_line = new.tree().span(*n).start_line;
            Placed {
                function: enclosing_function(new, *n, diff.lang),
                change: Change {
                    op: OpKind::Move,
                    node_kind: kind.clone(),
                    old_line: Some(old.tree().span(*o).start_line),
                    new_line: Some(new_line),
                    description: format!("`{}` moved to line {new_line}", snippet(new.text(*n))),
                },
            }
        }
        EditOp::Update {
            old: o,
            new: n,
            new_kind,
            ..
        } => Placed {
            function: enclosing_function(new, *n, diff.lang),
            change: Change {
                op: OpKind::Update,
                node_kind: new_kind.clone(),
                old_line: Some(old.tree().span(*o).start_line),
                new_line: Some(new.tree().span(*n).start_line),
                description: format!(
                    "Changed `{}` to `{}`",
                    snippet(old.text(*o)),
                    snippet(new.text(*n))
                ),
            },
        },
    }
}

/// The innermost function that contains `node`, counting `node` itself.
fn enclosing_function(
    file: &Normalized,
    node: NodeId,
    lang: SourceLang,
) -> Option<(String, Option<(usize, usize)>)> {
    let tree = file.tree();
    let mut cur = Some(node);
    while let Some(n) = cur {
        if lang.function_kinds().contains(&tree.kind(n)) {
            let name = tree
                .children(n)
                .iter()
                .find(|&&c| NAME_KINDS.contains(&tree.kind(c)))
                .and_then(|&c| tree.value(c))
                .unwrap_or("(anonymous)")
                .to_owned();
            let span = tree.span(n);
            return Some((name, Some((span.start_line, span.end_line))));
        }
        cur = tree.parent(n);
    }
    None
}

fn add_to_group(groups: &mut Vec<Group>, placed: Placed) {
    let name = placed.function.as_ref().map(|(name, _)| name.clone());
    let line = placed
        .change
        .new_line
        .or(placed.change.old_line)
        .unwrap_or(1);
    let index = match groups.iter().position(|g| g.function == name) {
        Some(index) => index,
        None => {
            let (start_line, end_line) = placed
                .function
                .as_ref()
                .and_then(|(_, lines)| *lines)
                .unwrap_or((line, line));
            groups.push(Group {
                function: name,
                start_line,
                end_line,
                changes: Vec::new(),
            });
            groups.len() - 1
        }
    };
    let group = &mut groups[index];
    if group.function.is_none() {
        group.start_line = group.start_line.min(line);
        group.end_line = group.end_line.max(line);
    }
    group.changes.push(placed.change);
}

/// First non-blank line of `text`, whitespace-collapsed and shortened for display.
pub fn snippet(text: &str) -> String {
    let first = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    let collapsed = first.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= SNIPPET_CHARS {
        collapsed
    } else {
        let head: String = collapsed.chars().take(SNIPPET_CHARS - 1).collect();
        format!("{head}…")
    }
}
