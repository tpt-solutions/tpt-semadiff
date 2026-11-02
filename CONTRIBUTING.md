# Contributing to tpt-semadiff

Thanks for looking at tpt-semadiff. Contributions are handled through issues only: open an
issue to report a bug or request a feature. Pull requests are not accepted. This guide covers
building and testing the project, so you can reproduce a problem before you report it.

## Workspace layout

| Crate | Purpose |
| --- | --- |
| `crates/tpt-semadiff-core` | The engine: parsing, normalization, matching, edit scripts, reports, summaries |
| `crates/tpt-semadiff-cli` | The `semadiff` binary: argument handling, directory and `--rev` modes, CI gates, `doctor` |
| `crates/tpt-semadiff-tui` | The interactive terminal viewer (ratatui) |

The design is described in [docs/architecture.md](docs/architecture.md). Output formats are
in [docs/json-schema.md](docs/json-schema.md) and the CLI reference is in
[docs/usage.md](docs/usage.md).

## Requirements

- A Rust toolchain at least as new as the `rust-version` in the workspace `Cargo.toml`
  (currently 1.90).
- A C compiler. The tree-sitter grammars are compiled from C sources.

## Build

```
cargo build --workspace
```

For an optimized binary, use `cargo build --release`. The binary is `target/release/semadiff`.

## Test

```
cargo test --workspace
```

Each crate has integration tests under its `tests/` directory. The core crate also has a
benchmark example in `crates/tpt-semadiff-core/examples/bench.rs`, which is not run by the
tests.

## Lint and format

CI runs these three commands. Run them before you push, and they should pass with no output
or warnings:

```
cargo build --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

The formatter settings are in `rustfmt.toml`, and the clippy settings are in `clippy.toml`.

## Adding a language

Languages are defined by the `SourceLang` enum in
`crates/tpt-semadiff-core/src/parse.rs`. To add one:

1. Add the tree-sitter grammar crate to `crates/tpt-semadiff-core/Cargo.toml`.
2. Add a variant to `SourceLang`, and add it to the `ALL` array. `semadiff doctor` loops over
   `ALL` to check that every grammar loads, so a missing entry is not caught there.
3. Map its file extensions in `SourceLang::from_path`.
4. Fill in `name`, `grammar`, `function_kinds`, `import_kinds` and `binding_kinds` with the
   node kinds from that grammar. `function_kinds` decides what counts as a named function
   for the per-function grouping.
5. If the language needs special handling in trivia normalization, see
   `crates/tpt-semadiff-core/src/normalize.rs`. Python docstrings are an example.
6. Add tests in `crates/tpt-semadiff-core/tests/languages.rs`, covering extension detection
   and at least one diff in the new language.
7. Update the language table in `docs/usage.md`, the list in `README.md`, and the
   `Unreleased` section of `CHANGELOG.md`.

## Reporting bugs and requesting features

Open an issue using the bug report or feature request template. For bugs, the template asks
for the `semadiff --version` output and the `semadiff doctor` output, which saves a round trip.

Do not report security problems in a public issue. See [SECURITY.md](SECURITY.md).
