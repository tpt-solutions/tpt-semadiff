# tpt-semadiff — Project TODO

Structural, AST-aware diff and merge tool (Rust). Dual-licensed MIT OR Apache-2.0. TPT Solutions.
Target release: Q3 2026. Tracking checklist — check items off as completed.

## Phase 0 — Project Setup & Licensing
- [x] Initialize git repository
- [x] Create Cargo workspace (e.g. `tpt-semadiff-core`, `tpt-semadiff-cli`, `tpt-semadiff-tui` crates)
- [x] Add `LICENSE-MIT` and `LICENSE-APACHE` (dual license, copyright TPT Solutions)
- [x] Add `README.md` with project overview, goals/non-goals from spec
- [x] Add `.gitignore`, `rustfmt.toml`, `clippy.toml`
- [x] Set up crate metadata (`Cargo.toml`: license = "MIT OR Apache-2.0", authors, description, repository)
- [x] Choose and vendor initial dependencies (tree-sitter, a TUI crate e.g. ratatui, serde/serde_json, clap)

## Phase 1 — Core Architecture (language-agnostic)
- [x] Define a generic tree interface/trait (node kind, children, span, value) decoupled from any specific language
- [x] Define the semantic operation model: Insert, Delete, Move, Update
- [x] Define internal IR for representing a parsed tree independent of Tree-sitter specifics
- [x] Write unit tests for the generic tree model using synthetic/mock trees (no real language grammar yet)

## Phase 2 — Tree-sitter Integration
- [x] Integrate `tree-sitter` crate and grammar-loading mechanism
- [x] Build adapter layer: Tree-sitter parse tree → internal generic tree IR
- [x] Add first real language grammar (choose based on dogfooding needs, e.g. Rust or Python)
- [x] Integration tests: parse real source files into internal IR correctly

## Phase 3 — CST Normalizer
- [x] Implement trivia stripping (whitespace, comments) for diffing purposes
- [x] Retain mapping from normalized tree back to original CST (for formatting-preserving reconstruction)
- [x] Tests: round-trip a file through normalize → reconstruct with no formatting loss

## Phase 4 — Tree Diff Engine: Zhang-Shasha baseline
- [x] Implement Zhang-Shasha tree edit distance algorithm over the internal IR
- [x] Convert raw edit distance operations into the semantic op model (Insert/Delete/Move/Update)
- [x] Correctness test suite (small/synthetic trees with known-minimal edit scripts)
- [x] Benchmark on real files to confirm expected scaling limits

## Phase 5 — Tree Diff Engine: Chawathe upgrade
- [x] Implement Chawathe et al. (GumTree-style) matching + edit script algorithm
- [x] Node matching heuristics (structural + content similarity)
- [x] Move detection across subtrees
- [x] Benchmark against Zhang-Shasha baseline on real-world file sizes; make Chawathe the default engine
- [x] Regression tests comparing outputs against Phase 4 baseline for known cases

## Phase 6 — Semantic Formatter / TUI
- [x] Design semantic op → human-readable grouping (by function/block, per spec's terminal output example)
- [x] Implement TUI renderer (boxed per-function change summary, MOVED/UPDATE/INSERT annotations)
- [x] Implement standard unified-diff-style text output mode
- [x] Color/formatting for terminal output
- [x] Manual + snapshot tests of formatter output against sample diffs

## Phase 7 — CLI & Flags
- [x] Set up `clap`-based CLI with subcommands/flags
- [x] Implement `--ignore-formatting` flag (report zero changes on pure reformatting)
- [x] Implement `--out json` flag (structured semantic diff export)
- [x] Define and document JSON schema for exported semantic diff
- [x] Tests for CLI flag behavior

## Phase 8 — Git Integration
- [x] Implement `git difftool`-compatible invocation interface (file pair args, exit codes)
- [ ] Document/script git config setup for using tpt-semadiff as difftool/mergetool
- [x] Test drop-in usage as a replacement for vimdiff/meld in a real git repo

## Phase 9 — AI-Refactor Summarizer (rule-based)
- [x] Design template/rule system mapping semantic op patterns → natural-language descriptions (e.g. "Extracted validation logic into a helper function")
- [x] Implement rule-based summarizer over semantic diff output
- [x] Tests covering common refactor patterns (extract function, rename, reorder imports, inline variable, etc.)

## Phase 10 — Expand Language Support
- [x] Add additional Tree-sitter grammars beyond the first (e.g. Python, JavaScript/TypeScript, Go, etc. — prioritize by user demand)
- [x] Per-language normalizer edge cases (comments/docstrings, language-specific trivia)
- [x] Per-language integration test suites

## Phase 11 — LLM-backed Summarizer (advanced, optional)
- [x] Design pluggable summarizer backend interface (rule-based vs LLM-backed)
- [x] Implement LLM-backed summarizer (API key config, network handling, cost/latency/error handling, offline fallback to rule-based)
- [x] Tests/mocking for LLM-backed path

## Phase 12 — Testing, CI/CD & Distribution
- [x] Set up GitHub Actions CI (build, test, clippy, fmt check) across Linux/macOS/Windows
- [x] Set up code coverage reporting
- [x] Configure crates.io publishing workflow
- [x] Configure prebuilt binary releases (e.g. cargo-dist) for GitHub Releases across platforms
- [x] Add CI enforcement example: "no PR modifies >3 semantic functions without human review" using `--out json`

## Phase 13 — Documentation & Release
- [x] Write user documentation (installation, usage, git integration setup, CLI flag reference)
- [x] Write architecture/contributor documentation
- [x] Write CHANGELOG.md
- [ ] Tag and publish v0.1.0 (initial public release)
- [ ] Plan and execute Q3 2026 target release milestone
