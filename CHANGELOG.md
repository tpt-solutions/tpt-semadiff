# Changelog

## Unreleased

Initial implementation, not yet released.

- Parsing with tree-sitter for Rust, Python, JavaScript and Go.
- Trivia normalization with a reconstructable segment list; `--ignore-formatting`.
- Chawathe/GumTree-style matcher (default) and a Zhang-Shasha engine.
- Semantic report grouped by function; boxed text, unified and JSON output.
- Interactive TUI viewer.
- Git difftool and external-diff invocation.
- Rule-based summaries (rename, import reorder, extract function, inline variable), plus
  an optional LLM summarizer with offline fallback.
