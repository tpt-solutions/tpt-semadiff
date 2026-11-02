# Architecture

## Pipeline

```
old file ─┐                                        ┌─ report.rs (grouped by function)
          ├─ parse.rs ─ normalize.rs ─ gumtree.rs ─┼─ ops.rs (edit script)  ─┼─ render.rs (text, unified)
new file ─┘  (tree-sitter)  (trivia)   (matching)  └─                       └─ tpt-semadiff-tui, JSON
                                      zhang_shasha.rs (alternative engine)
```

## Modules (crate `tpt-semadiff-core`)

- `tree.rs`: the `Tree` trait. Algorithms only see this, never tree-sitter.
- `ir.rs`: `ArenaTree`, the owned tree every front end lowers into. Nodes have a kind, an
  optional value (text for leaves), a span, and a named flag.
- `parse.rs`: tree-sitter front end and the per-language tables (function, import and
  binding kinds).
- `normalize.rs`: trivia policy. Keeps a segment list that covers the whole file, so the
  original bytes can always be rebuilt. Maps normalized nodes back to the concrete tree.
- `mapping.rs`: one-to-one node correspondence.
- `gumtree.rs`: default matcher. Strict top-down on unique identical subtrees; Dice-based
  container matching (0.5, then a relaxed 0.3 only when parents correspond); loose top-down
  for remaining identical subtrees, parent-aware; then recovery under matched parents.
- `zhang_shasha.rs`: Zhang-Shasha edit distance with mapping recovery. Exact, O(n²m²).
- `ops.rs`: derives Insert, Delete, Move and Update from a mapping. Moves come from a
  longest common subsequence of siblings. Inserts and deletes are reported per maximal
  unmatched subtree.
- `report.rs`: groups operations by enclosing function. Serializable.
- `render.rs`: boxed text and unified diff output.
- `summarize/`: `rules.rs` (offline recognizers) and `llm.rs` (model-backed, with fallback).

## Known limitations

- Matching is heuristic. Large restructurings, such as extracting a function whose body
  also changed, can pair nodes differently from what a person would. The summary rules
  match statement text, so they are less sensitive to this than the box output.
- Some anonymous punctuation can appear as an insert and a delete when its parent was
  rematched. These are real token differences, but they are noisy.
- Adding or removing an optional semicolon is a token change, and is reported.
- Only the four grammars listed in the README are supported.
