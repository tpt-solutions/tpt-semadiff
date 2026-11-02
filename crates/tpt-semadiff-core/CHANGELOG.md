# Changelog

All notable changes to `tpt-semadiff-core` are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Unreleased

Initial implementation, not yet released.

### Added

- Parsing with tree-sitter for Rust, Python, JavaScript, TypeScript (`.ts`, `.mts`, `.cts`),
  TSX (`.tsx`) and Go, through `parse::SourceLang` and `parse::parse_tree`.
- `SourceLang::from_path` chooses the language from a file extension.
- A generic `Tree` trait, so matching algorithms work on any tree, and an owned `ArenaTree`
  that every front end lowers into.
- Trivia normalization with a reconstructable segment list, and `TriviaPolicy::Strip` for
  comment-insensitive diffs (`Options::ignore_formatting`).
- GumTree-style matcher with a Chawathe edit script (`Engine::Chawathe`, the default).
- Zhang-Shasha tree edit distance with mapping recovery (`Engine::ZhangShasha`).
- Edit operations `Insert`, `Delete`, `Move` and `Update` derived from a mapping, with moves
  found from a longest common subsequence of siblings.
- `diff::diff_files` runs the whole pipeline on two source texts; `diff::diff_trees` runs
  matching and the script on two prebuilt trees.
- `report::Report` groups changes by enclosing function, with counts and line numbers, and
  serializes with serde (`to_json`).
- Renderers: boxed text (`render_report`), unified diff (`render_unified`) and Markdown for
  pull request comments (`render_markdown`).
- Offline rule-based summaries (`summarize::rules`): rename, import reorder, extract function
  and inline variable.
- Optional model-backed summarizer (`summarize::llm`) with `AnthropicClient`, falling back to
  the rules on any failure (`summarize_with_fallback`).
