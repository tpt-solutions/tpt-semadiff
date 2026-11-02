//! Zhang-Shasha tree edit distance (SIAM J. Computing, 1989), with mapping recovery.
//!
//! Costs: insert and delete cost 1, and a relabel costs 1 when kind or value differ.
//! The recovered mapping realizes the minimum cost; ties are broken arbitrarily.
//! Worst case is O(n² m²), so this is the baseline engine rather than the default.

use crate::mapping::Mapping;
use crate::tree::{NodeId, Tree};

/// A tree prepared for Zhang-Shasha. Indices are 1-based post-order positions.
struct Indexed {
    post: Vec<NodeId>,
    /// `lml[i]`: post-order index of the leftmost leaf under node `i`.
    lml: Vec<usize>,
    keyroots: Vec<usize>,
}

impl Indexed {
    fn new<T: Tree>(tree: &T) -> Self {
        let post = tree.postorder();
        let n = post.len();
        let mut position = vec![0; tree.node_count()];
        for (i, &node) in post.iter().enumerate() {
            position[node] = i + 1;
        }
        let mut lml = vec![0; n + 1];
        for i in 1..=n {
            lml[i] = match tree.children(post[i - 1]).first() {
                Some(&first) => lml[position[first]],
                None => i,
            };
        }
        let mut seen = vec![false; n + 1];
        let mut keyroots = Vec::new();
        for i in (1..=n).rev() {
            if !seen[lml[i]] {
                seen[lml[i]] = true;
                keyroots.push(i);
            }
        }
        keyroots.reverse();
        Self {
            post,
            lml,
            keyroots,
        }
    }

    fn node(&self, i: usize) -> NodeId {
        self.post[i - 1]
    }
}

struct Zs<'a, A: Tree, B: Tree> {
    a: &'a A,
    b: &'a B,
    ia: Indexed,
    ib: Indexed,
    /// Tree distances between pairs of subtrees, indexed `[i][j]` by post-order position.
    td: Vec<Vec<usize>>,
}

/// Computes the tree edit distance between `a` and `b`, and a mapping that realizes it.
pub fn mapping<A: Tree, B: Tree>(a: &A, b: &B) -> (usize, Mapping) {
    let ia = Indexed::new(a);
    let ib = Indexed::new(b);
    let (n, m) = (ia.post.len(), ib.post.len());
    let mut zs = Zs {
        a,
        b,
        ia,
        ib,
        td: vec![vec![0; m + 1]; n + 1],
    };
    for k in 0..zs.ia.keyroots.len() {
        for l in 0..zs.ib.keyroots.len() {
            let (i, j) = (zs.ia.keyroots[k], zs.ib.keyroots[l]);
            zs.forest(i, j);
        }
    }
    let mut pairs = Vec::new();
    zs.trace(n, m, &mut pairs);
    let mut map = Mapping::new(a.node_count(), b.node_count());
    for (i, j) in pairs {
        map.link(zs.ia.node(i), zs.ib.node(j));
    }
    (zs.td[n][m], map)
}

impl<A: Tree, B: Tree> Zs<'_, A, B> {
    fn rename_cost(&self, i: usize, j: usize) -> usize {
        let (x, y) = (self.ia.node(i), self.ib.node(j));
        usize::from(self.a.kind(x) != self.b.kind(y) || self.a.value(x) != self.b.value(y))
    }

    /// Forest-distance table for the subtrees rooted at `i` and `j`. Records the tree
    /// distance of every pair of full subtrees it passes through.
    ///
    /// The entry for forest prefix `(a, b)` sits at row `a + 1 - li` and column
    /// `b + 1 - lj`, where `li` and `lj` are the leftmost-leaf indices. Row or column 0
    /// stands for the empty forest.
    fn forest(&mut self, i: usize, j: usize) -> Vec<usize> {
        let (li, lj) = (self.ia.lml[i], self.ib.lml[j]);
        let rows = i + 2 - li;
        let cols = j + 2 - lj;
        let mut fd = vec![0; rows * cols];
        for a in (li - 1)..=i {
            for b in (lj - 1)..=j {
                let (r, c) = (a + 1 - li, b + 1 - lj);
                let value = if a < li && b < lj {
                    0
                } else if a < li {
                    fd[r * cols + c - 1] + 1
                } else if b < lj {
                    fd[(r - 1) * cols + c] + 1
                } else {
                    let delete = fd[(r - 1) * cols + c] + 1;
                    let insert = fd[r * cols + c - 1] + 1;
                    let full = self.ia.lml[a] == li && self.ib.lml[b] == lj;
                    let matched = if full {
                        fd[(r - 1) * cols + c - 1] + self.rename_cost(a, b)
                    } else {
                        let (la, lb) = (self.ia.lml[a], self.ib.lml[b]);
                        fd[(la - li) * cols + (lb - lj)] + self.td[a][b]
                    };
                    let value = delete.min(insert).min(matched);
                    if full {
                        self.td[a][b] = value;
                    }
                    value
                };
                fd[r * cols + c] = value;
            }
        }
        fd
    }

    /// Walks back through the forest table of `(i, j)` and collects matched pairs.
    fn trace(&mut self, i: usize, j: usize, pairs: &mut Vec<(usize, usize)>) {
        let (li, lj) = (self.ia.lml[i], self.ib.lml[j]);
        let cols = j + 2 - lj;
        let fd = self.forest(i, j);
        let at = |a: usize, b: usize| fd[(a + 1 - li) * cols + (b + 1 - lj)];

        let (mut a, mut b) = (i, j);
        loop {
            if a < li && b < lj {
                break;
            }
            if a < li {
                b -= 1;
            } else if b < lj || at(a, b) == at(a - 1, b) + 1 {
                a -= 1;
            } else if at(a, b) == at(a, b - 1) + 1 {
                b -= 1;
            } else if self.ia.lml[a] == li && self.ib.lml[b] == lj {
                pairs.push((a, b));
                a -= 1;
                b -= 1;
            } else {
                let (la, lb) = (self.ia.lml[a], self.ib.lml[b]);
                self.trace(a, b, pairs);
                a = la - 1;
                b = lb - 1;
            }
        }
    }
}
