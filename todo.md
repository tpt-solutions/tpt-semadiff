# tpt-semadiff — Project TODO

Structural, AST-aware diff and merge tool (Rust). Dual-licensed MIT OR Apache-2.0. TPT Solutions.
Target release: Q3 2026. Tracking checklist — check items off as completed.

## Phase 0 — Project Setup & Licensing
- [ ] Initialize git repository
- [ ] Create Cargo workspace (e.g. `semadiff-core`, `semadiff-cli`, `semadiff-tui` crates)
- [ ] Add `LICENSE-MIT` and `LICENSE-APACHE` (dual license, copyright TPT Solutions)
- [ ] Add `README.md` with project overview, goals/non-goals from spec
- [ ] Add `.gitignore`, `rustfmt.toml`, `clippy.toml`
- [ ] Set up crate metadata (`Cargo.toml`: license = "MIT OR Apache-2.0", authors, description, repository)
- [ ] Choose and vendor initial dependencies (tree-sitter, a TUI crate e.g. ratatui, serde/serde_json, clap)

## Phase 1 — Core Architecture (language-agnostic)
- [ ] Define a generic tree interface/trait (node kind, children, span, value) decoupled from any specific language
- [ ] Define the semantic operation model: Insert, Delete, Move, Update
- [ ] Define internal IR for representing a parsed tree independent of Tree-sitter specifics
- [ ] Write unit tests for the generic tree model using synthetic/mock trees (no real language grammar yet)

## Phase 2 — Tree-sitter Integration
- [ ] Integrate `tree-sitter` crate and grammar-loading mechanism
- [ ] Build adapter layer: Tree-sitter parse tree → internal generic tree IR
- [ ] Add first real language grammar (choose based on dogfooding needs, e.g. Rust or Python)
- [ ] Integration tests: parse real source files into internal IR correctly

## Phase 3 — CST Normalizer
- [ ] Implement trivia stripping (whitespace, comments) for diffing purposes
- [ ] Retain mapping from normalized tree back to original CST (for formatting-preserving reconstruction)
- [ ] Tests: round-trip a file through normalize → reconstruct with no formatting loss

## Phase 4 — Tree Diff Engine: Zhang-Shasha baseline
- [ ] Implement Zhang-Shasha tree edit distance algorithm over the internal IR
- [ ] Convert raw edit distance operations into the semantic op model (Insert/Delete/Move/Update)
- [ ] Correctness test suite (small/synthetic trees with known-minimal edit scripts)
- [ ] Benchmark on real files to confirm expected scaling limits

## Phase 5 — Tree Diff Engine: Chawathe upgrade
- [ ] Implement Chawathe et al. (GumTree-style) matching + edit script algorithm
- [ ] Node matching heuristics (structural + content similarity)
- [ ] Move detection across subtrees
- [ ] Benchmark against Zhang-Shasha baseline on real-world file sizes; make Chawathe the default engine
- [ ] Regression tests comparing outputs against Phase 4 baseline for known cases

## Phase 6 — Semantic Formatter / TUI
- [ ] Design semantic op → human-readable grouping (by function/block, per spec's terminal output example)
- [ ] Implement TUI renderer (boxed per-function change summary, MOVED/UPDATE/INSERT annotations)
- [ ] Implement standard unified-diff-style text output mode
- [ ] Color/formatting for terminal output
- [ ] Manual + snapshot tests of formatter output against sample diffs

## Phase 7 — CLI & Flags
- [ ] Set up `clap`-based CLI with subcommands/flags
- [ ] Implement `--ignore-formatting` flag (report zero changes on pure reformatting)
- [ ] Implement `--out json` flag (structured semantic diff export)
- [ ] Define and document JSON schema for exported semantic diff
- [ ] Tests for CLI flag behavior

## Phase 8 — Git Integration
- [ ] Implement `git difftool`-compatible invocation interface (file pair args, exit codes)
- [ ] Document/script git config setup for using tpt-semadiff as difftool/mergetool
- [ ] Test drop-in usage as a replacement for vimdiff/meld in a real git repo

## Phase 9 — AI-Refactor Summarizer (rule-based)
- [ ] Design template/rule system mapping semantic op patterns → natural-language descriptions (e.g. "Extracted validation logic into a helper function")
- [ ] Implement rule-based summarizer over semantic diff output
- [ ] Tests covering common refactor patterns (extract function, rename, reorder imports, inline variable, etc.)

## Phase 10 — Expand Language Support
- [ ] Add additional Tree-sitter grammars beyond the first (e.g. Python, JavaScript/TypeScript, Go, etc. — prioritize by user demand)
- [ ] Per-language normalizer edge cases (comments/docstrings, language-specific trivia)
- [ ] Per-language integration test suites

## Phase 11 — LLM-backed Summarizer (advanced, optional)
- [ ] Design pluggable summarizer backend interface (rule-based vs LLM-backed)
- [ ] Implement LLM-backed summarizer (API key config, network handling, cost/latency/error handling, offline fallback to rule-based)
- [ ] Tests/mocking for LLM-backed path

## Phase 12 — Testing, CI/CD & Distribution
- [ ] Set up GitHub Actions CI (build, test, clippy, fmt check) across Linux/macOS/Windows
- [ ] Set up code coverage reporting
- [ ] Configure crates.io publishing workflow
- [ ] Configure prebuilt binary releases (e.g. cargo-dist) for GitHub Releases across platforms
- [ ] Add CI enforcement example: "no PR modifies >3 semantic functions without human review" using `--out json`

## Phase 13 — Documentation & Release
- [ ] Write user documentation (installation, usage, git integration setup, CLI flag reference)
- [ ] Write architecture/contributor documentation
- [ ] Write CHANGELOG.md
- [ ] Tag and publish v0.1.0 (initial public release)
- [ ] Plan and execute Q3 2026 target release milestone
