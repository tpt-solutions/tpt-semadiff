# tpt-semadiff-core

[![Crates.io](https://img.shields.io/crates/v/tpt-semadiff-core.svg)](https://crates.io/crates/tpt-semadiff-core)
[![docs.rs](https://img.shields.io/docsrs/tpt-semadiff-core)](https://docs.rs/tpt-semadiff-core)

The diffing engine behind [tpt-semadiff](https://github.com/tpt-solutions/tpt-semadiff). It
compares two versions of a source file by their syntax trees rather than by lines, so a
reformatting, a comment edit or a moved function shows up as what it is.

This crate is a library. It has no command-line interface and no terminal output of its own.
Most users want the [`semadiff`](https://github.com/tpt-solutions/tpt-semadiff/tree/master/crates/tpt-semadiff-cli)
command instead.

## What it does

1. **Parses** source text with [tree-sitter](https://tree-sitter.github.io/) into an owned
   arena tree (`ir::ArenaTree`).
2. **Normalizes** trivia. Comments and whitespace are kept as a segment list that covers the
   whole file, so the original bytes can always be rebuilt. Optionally, comments are stripped
   so that comment-only edits report zero changes.
3. **Matches** nodes between the old and new trees with one of two engines:
   - `Engine::Chawathe` (the default): GumTree-style top-down and bottom-up matching with a
     Chawathe edit script. Fast enough for large files.
   - `Engine::ZhangShasha`: Zhang-Shasha minimum-cost tree edit mapping. Exact, but
     O(n²m²) in the worst case, so it is best for small inputs and benchmarks.
4. **Derives** a semantic edit script of `Insert`, `Delete`, `Move` and `Update` operations
   (`ops::EditOp`).
5. **Reports** the script grouped by enclosing function (`report::Report`), and renders it as
   boxed text, unified diff, Markdown or JSON.
6. **Summarizes** the changes in plain English, either with offline rules
   (`summarize::rules`: rename, import reorder, extract function, inline variable) or with an
   optional model-backed summarizer (`summarize::llm`) that falls back to the rules on any
   failure.

## Supported languages

| Language | `SourceLang` | Extensions |
| --- | --- | --- |
| Rust | `Rust` | `.rs` |
| Python | `Python` | `.py` |
| JavaScript | `JavaScript` | `.js`, `.mjs`, `.cjs`, `.jsx` |
| TypeScript | `TypeScript` | `.ts`, `.mts`, `.cts` |
| TSX | `Tsx` | `.tsx` |
| Go | `Go` | `.go` |

`SourceLang::from_path` picks the language from a file extension. Any other file returns
`None`, and callers decide how to handle it.

## Installation

```
cargo add tpt-semadiff-core
```

The crate needs a C compiler, because tree-sitter grammars are compiled from C sources.

## Example

Diff two Rust sources and print the changed functions:

```rust
use std::path::Path;

use tpt_semadiff_core::diff::{diff_files, Options};
use tpt_semadiff_core::parse::SourceLang;
use tpt_semadiff_core::report::Report;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let old = "fn total(xs: &[u32]) -> u32 {\n    let mut sum = 0;\n    for x in xs { sum += x; }\n    sum\n}\n";
    let new = "fn total(xs: &[u32]) -> u32 {\n    let mut acc = 0;\n    for x in xs { acc += x; }\n    acc\n}\n";

    let lang = SourceLang::from_path(Path::new("lib.rs")).ok_or("unsupported language")?;
    let options = Options::default();
    let diff = diff_files(old, new, lang, options)?;
    let report = Report::from_diff(&diff, "lib.rs (old)", "lib.rs (new)", options.engine.name());

    for name in report.changed_functions() {
        println!("changed: {name}");
    }
    println!("{}", report.to_json()?);
    Ok(())
}
```

Comment-only edits can be ignored:

```rust
let options = Options {
    ignore_formatting: true,
    ..Options::default()
};
```

To compare the matching engines on the same input, set `Options::engine` to
`Engine::ZhangShasha`.

## Public API

| Module | Main items | Purpose |
| --- | --- | --- |
| `diff` | `diff_files`, `diff_trees`, `Options`, `Engine`, `FileDiff` | The whole pipeline, from two texts to an edit script |
| `parse` | `SourceLang`, `parse_tree`, `ParseError` | tree-sitter front end and per-language node tables |
| `normalize` | `normalize`, `TriviaPolicy`, `Normalized`, `Segment` | Trivia handling with a reconstructable segment list |
| `ir` | `ArenaTree` | Owned tree every front end lowers into |
| `tree` | `Tree`, `Span`, `NodeId` | Trait the matching algorithms are written against |
| `mapping` | `Mapping` | One-to-one node correspondence between old and new |
| `gumtree` | `mapping` | Default GumTree-style matcher |
| `zhang_shasha` | `mapping` | Zhang-Shasha edit distance with mapping recovery |
| `ops` | `EditOp`, `script_from_mapping` | Insert, delete, move and update derived from a mapping |
| `report` | `Report`, `Group`, `Change`, `OpKind`, `ChangeCounts` | Changes grouped by function; serializable with serde |
| `render` | `render_report`, `render_group`, `render_markdown`, `render_unified` | Text, Markdown and unified diff output |
| `summarize` | `rules::summarize`, `llm::summarize_with_fallback`, `llm::AnthropicClient` | Plain-English summaries |

Algorithms are written against the `Tree` trait, not against tree-sitter. Any tree that
implements `Tree` can be matched with `diff::diff_trees`. This is how the matchers are tested
on synthetic trees.

## Known limitations

- Matching is heuristic. Large restructurings, such as extracting a function whose body also
  changed, can pair nodes differently from what a person would expect.
- Adding or removing an optional semicolon is a real token change, and is reported.
- Some anonymous punctuation can appear as an insert and a delete when its parent was
  rematched. These are real token differences, but they are noisy.
- A file that is new on one side shows a top-level update from an empty string to its first
  line. The insert shows the real change.

See [docs/architecture.md](https://github.com/tpt-solutions/tpt-semadiff/blob/master/docs/architecture.md)
for the module-by-module design and
[docs/benchmarks.md](https://github.com/tpt-solutions/tpt-semadiff/blob/master/docs/benchmarks.md)
for measured performance.

## Stability

The crate is pre-1.0. Public types may change between minor releases until 1.0. The
[changelog](CHANGELOG.md) lists every change.

## Minimum supported Rust version

1.90.

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-semadiff/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-semadiff/blob/master/LICENSE-APACHE), at
your option. Copyright TPT Solutions.
