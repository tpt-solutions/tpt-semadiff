# tpt-semadiff-tui

[![Crates.io](https://img.shields.io/crates/v/tpt-semadiff-tui.svg)](https://crates.io/crates/tpt-semadiff-tui)
[![docs.rs](https://img.shields.io/docsrs/tpt-semadiff-tui)](https://docs.rs/tpt-semadiff-tui)

An interactive terminal viewer for the semantic diff reports produced by
[tpt-semadiff](https://github.com/tpt-solutions/tpt-semadiff). It is built on
[ratatui](https://ratatui.rs/).

The viewer shows one entry per function that changed. Pick a function on the left and its
changes appear on the right, so a long review can be walked through one function at a time
instead of scrolling a flat list.

```
┌ src/main.rs (3 changes) ┐┌ calculate_metrics (Lines 40-85) ──────────┐
│> calculate_metrics      ││ [UPDATE] Changed `sum` to `acc`           │
│  parse_header           ││ [INSERT] Added `if metrics.is_empty() {`  │
│  (top level)            ││                                           │
└─────────────────────────┘└───────────────────────────────────────────┘
```

This crate is a library. It is used by the `semadiff` binary, which starts the viewer with
`semadiff --tui OLD NEW`. You can also embed the viewer in your own terminal application.

## Installation

```
cargo add tpt-semadiff-tui
```

The viewer needs a terminal that supports the alternate screen and raw mode. Any terminal
that runs ratatui's crossterm backend works, including Windows Terminal, the Windows console,
macOS Terminal, iTerm2 and common Linux terminals.

## Usage

From the command line, through the `semadiff` binary:

```
semadiff --tui OLD.rs NEW.rs
```

The viewer works on one file pair at a time.

From Rust:

```rust
use tpt_semadiff_core::report::Report;

fn browse(report: &Report) -> std::io::Result<()> {
    tpt_semadiff_tui::run(report)
}
```

`run` takes over the terminal, draws until the user quits, then restores the terminal. It
returns when the user quits, or with an error if the terminal cannot be initialized or read.

To build the report, use `tpt_semadiff_core::diff::diff_files` followed by
`tpt_semadiff_core::report::Report::from_diff`. See the
[core crate](https://crates.io/crates/tpt-semadiff-core) for the details.

## Keys

| Key | Action |
| --- | --- |
| `j` or `↓` | Move down to the next function |
| `k` or `↑` | Move up to the previous function |
| `q` or `Esc` | Quit |

When the report has no changes, the viewer opens with an empty list and quits on `q` or `Esc`.

## Public API

| Item | Purpose |
| --- | --- |
| `run(report: &Report) -> io::Result<()>` | Runs the viewer until the user quits. Initializes and restores the terminal |
| `draw(frame, report, state)` | Draws one frame into a ratatui `Frame`. Use it to place the viewer inside a larger layout |

`draw` takes a `ListState` that holds the current selection. Keep it across frames so the
selection survives redraws.

## Limitations

- The viewer is read-only. It does not apply, stage or revert changes.
- It shows one file pair. Directory and revision modes print their reports to the terminal
  instead.
- Matching is heuristic, so a change can be grouped under a different function than a person
  would expect. See the [core crate's limitations](https://crates.io/crates/tpt-semadiff-core).

## Minimum supported Rust version

1.90.

## License

Dual-licensed under [MIT](https://github.com/tpt-solutions/tpt-semadiff/blob/master/LICENSE-MIT)
or [Apache-2.0](https://github.com/tpt-solutions/tpt-semadiff/blob/master/LICENSE-APACHE), at
your option. Copyright TPT Solutions.
