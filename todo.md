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
- [x] Document/script git config setup for using tpt-semadiff as difftool/mergetool
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

## Phase 14 — Adoption & Usability
- [x] Add TypeScript support (`.ts`, `.mts`, `.cts`, `.tsx`) via tree-sitter-typescript, with tests in `tests/languages.rs`
- [x] Restructure CLI into optional subcommands without breaking `semadiff OLD NEW`
- [x] `semadiff git-setup` / `--uninstall`: write difftool config (`--external`, `--local`/`--global`), idempotent; closes the Phase 8 git-config scripting item
- [x] Git revision mode: `semadiff --rev HEAD~1` / `A..B` diffs all changed supported files (via `git diff --name-status -z` + `git show`)
- [x] Directory-to-directory diff for non-git use
- [x] Multi-file output: per-file grouping in text, JSON `{files: [...]}` (document in docs/json-schema.md; single-file shape unchanged)
- [x] Built-in CI gate: `--max-functions N` and `--fail-on-unparseable` (exit 1 on breach); rewrite docs/ci-enforcement.md around it, keep shell script as fallback
- [x] `--out markdown` renderer for GitHub PR comments, plus a copy-paste GitHub Actions snippet in the docs
- [x] `semadiff completions <shell>` (bash, zsh, fish, powershell) via clap_complete
- [x] `semadiff doctor`: check git on PATH, difftool config, API key set/unset (never print it), built-in grammars
- [x] Default `--summary rules` when stdout is a terminal
- [x] cargo-dist installers (`shell`, `powershell`), regenerate release workflow, add Install section to README
- [x] Verify new dependencies build on MSRV (bumped to 1.90 for current dependency tree)
- [x] CLI tests using a temp git repo (git-setup, `--rev`, `--max-functions`, markdown snapshot)
- [x] Update README, docs/usage.md, docs/git-integration.md; add CHANGELOG `Unreleased` entry

## Phase 15 — Spec gaps & rough edges (core)
- [x] Semantic unified output: group `--out unified` hunks by function/class block with headers (render.rs `render_unified` is currently a plain line diff)
- [x] Function detection: add arrow functions, function expressions, Go func literals, Rust closures to `function_kinds` (parse.rs) so `--max-functions` and grouping are accurate
- [x] Summarizer rules: share name lookup with report.rs (`property_identifier`, `field_identifier`); per-language body kinds instead of `kind.contains("block")`; widen `inline_lines` beyond literal `=`
- [x] LLM summary visibility: stop discarding `Summary.source` (pipeline.rs), report why a rules fallback happened, make `max_tokens`/system prompt overridable via env, verify default model id
- [x] Make `ureq` an optional `llm` feature (default on in CLI); centralize grammar/dep versions in `[workspace.dependencies]`
- [x] Wire `Normalized::reconstruct`/`segments`/`cst`/`origin` into a real consumer or document as library-only; remove unused `Mapping::len`/`is_empty`
- [x] `--ignore-formatting`: also ignore trailing-comma/paren-only changes; document the exact guarantee

## Phase 16 — Adoption tooling
- [x] `.semadiff.toml` project config (discovered upward from cwd; flags override) and `semadiff init` template writer
- [x] Directory/`--rev` excludes: honor `.gitignore`, add `--exclude GLOB`, default-skip lockfiles, `vendor/`, `node_modules/`, minified/generated files
- [x] `--lang <name>` override, extra extension mappings in config, shebang sniffing for extensionless scripts
- [x] `.gitattributes` per-path diff driver: `git-setup --driver`, docs for `*.rs diff=semadiff`; also set `difftool.prompt=false`
- [ ] `.pre-commit-hooks.yaml`, optional `git semadiff` alias, and `--quiet`/`--stat`/`--name-only` output modes for hooks
- [x] Formal JSON Schema file (`docs/semadiff.schema.json`) plus `schema_version` in `--out json`, validated by a test
- [x] Man page via `clap_mangen` (`semadiff man`) and per-shell completion install docs
- [x] Parallel multi-file diffing in `run_multi`; size/time warning for `--engine zhang-shasha` on large inputs
- [ ] cargo-dist: add Homebrew and npm installers, musl Linux targets; `cargo-binstall` metadata; dependabot config
- [x] Docs/community: CONTRIBUTING.md, SECURITY.md, issue/PR templates, README quickstart with real output, "vs difftastic/diffsitter" comparison, privacy note for `--summary llm`
- [x] Release prep: move CHANGELOG `Unreleased` to 0.1.0, make README install section unconditional, confirm publish.yml order/secrets (user tags v0.1.0)

## Later / ideas (not scheduled)
- [ ] More grammars: JSON, YAML, TOML, Java, C, C++
- [ ] Merge driver / mergetool (currently a documented non-goal)
- [ ] VS Code extension / LSP server using the JSON output
- [ ] Reusable GitHub Action wrapping the CI gate
- [ ] pre-commit hook config
- [x] `.semadiff.toml` project config file
- [ ] SARIF output
- [ ] Homebrew / winget packaging
