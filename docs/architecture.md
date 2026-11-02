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
- Only the six built-in grammars are supported: Rust, Python, JavaScript, TypeScript, TSX
  and Go. `semadiff doctor` checks that each one loads.
- For a file that is new on one side, the top-level box shows an update from an empty
  string to the first line. The insert shows the real change; the update is a side effect of
  matching an empty root node.

## CLI (crate `tpt-semadiff-cli`)

- `main.rs`: flags, subcommands, exit status and the CI gates.
- `pipeline.rs`: one old/new pair becomes a report, or a fallback when it cannot be parsed.
  Every input mode goes through it.
- `dirs.rs`: directory mode. Pairs supported files by relative path.
- `rev.rs`: revision mode. Asks git which files changed and reads both sides with `git show`.
- `output.rs`: text, unified, JSON and Markdown renderers, including the `{"files": [...]}`
  envelope.
- `setup.rs`: `git-setup`. Reads and writes git config through `git config`.
- `doctor.rs`: `doctor`.

## Feature flags and library-only API

- The `llm` feature of `tpt-semadiff-core` adds the Anthropic client and the LLM summarizer,
  and the `ureq` HTTP dependency with it. It is off by default, and the CLI turns it on. The TUI
  and other library users get the offline build.
- `Normalized::reconstruct`, `segments`, `cst` and `origin` are library API. The CLI does not
  use them. They are kept so that a tool can map a node back to its source or rebuild a file
  byte for byte. Tests cover `reconstruct`.
