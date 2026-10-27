# Terminal interaction contract

Date: 2026-09-28

Status: initial implementation decision. Tags: #tui #ux

Target 80 columns × 25 rows, with a clear resize message on smaller terminals.
Ratatui handles layout and drawing; Crossterm supplies keyboard events. The initial
screen is a setup dashboard, not a simulated live show.

## Ownership

- Never enable mouse capture by default.
- No clipboard reads/writes, OSC52 clipboard writes, or application-owned selection.
- Leave Ctrl+click links, context menus, and selection to the terminal emulator.
- No attempt to launch a browser on the Raspberry Pi for a link clicked on an SSH client.
- q, Esc and Ctrl+C exit; use Ctrl+C to exit, never to copy a selection.
- Restore terminal state on ordinary exit, returned errors, and Rust unwinding panics.
  SIGKILL and abrupt power loss cannot run cleanup.

The starter uses Ratatui's standard alternate screen and raw keyboard mode. Alternate
screen drawing does not require mouse capture. It does not yet provide scrollback/inline
mode. Plain `doctor` output works without a TUI and is suitable for redirection.
Test actual Konsole selection/link behavior interactively; PTY checks cannot verify a
local graphical clipboard over SSH.

The TUI is a view/controller of state, not the real-time lighting scheduler. Make
blackout and manual ownership visible when those controls exist; never imply they
are implemented with an inactive button or fabricated meter.

References: [Ratatui](https://docs.rs/ratatui/0.30.2/ratatui/),
[Crossterm](https://docs.rs/crossterm/0.29.0/crossterm/).
Related: [architecture](0005-architecture.md).

[Notebook index](../reference/NOTEBOOK.md)
