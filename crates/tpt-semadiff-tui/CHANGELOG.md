# Changelog

All notable changes to `tpt-semadiff-tui` are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Unreleased

Initial implementation, not yet released.

### Added

- `run(report)` opens an interactive viewer for a semantic diff report. It lists the changed
  functions on the left and their changes on the right.
- Keys: `j`/`k` and the arrow keys move between functions, `q` and `Esc` quit.
- `draw(frame, report, state)` renders one frame into a caller's ratatui layout.
- The `semadiff --tui` flag uses this viewer.
