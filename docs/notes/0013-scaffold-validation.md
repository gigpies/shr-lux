# Scaffold validation

Date: 2026-09-28
Status: measured locally on Raspberry Pi 5 / Debian 13 / AArch64.
Tags: #testing #evidence

- Formatting and Clippy with warnings denied passed.
- All five Rust tests passed: DMX address boundaries, range encoding, invalid ranges,
  80x25 display and undersized display.
- Optimized release build passed with the pinned Rust 1.97.1 toolchain.
- Linux PTY checks passed for q, Escape and Ctrl+C: clean exit, original termios restored,
  alternate screen left, no mouse-capture or OSC52 clipboard-enable output.
- Release doctor identified the actual USB candidate; normal-user access was denied as
  expected. Running only the descriptor diagnostic under sudo read the expected
  manufacturer, product and serial successfully. No vendor or DMX writes were sent.
- zk indexing, text search and new-note template dry-run passed. Relative links in
  notebook documents and README resolve. The template's links target its generated
  location under notes, not the template directory.
- udevadm verify accepted the supplied rule. It remains uninstalled.

Physical fixtures, DMX line timing, audio capture and MIDI/LED tests were intentionally
not run: the fixture hardware is unavailable and those paths are not implemented.
There are no historical or exhaustive tests in the scaffold.

The PTY suite cannot confirm graphical clipboard or Ctrl+click behavior in the user's
local Konsole. That remains an interactive check; the app emits no mouse capture commands.

[Development commands](0012-development-and-validation.md) · [Notebook index](../reference/NOTEBOOK.md)

GitHub repository creation succeeded as PRIVATE. Uploading an active workflow was
rejected because the OAuth token lacks workflow scope; CI is retained as an inactive
template. No remote CI run is claimed.

## CI authorization follow-up — 2026-09-28

GitHub device authorization completed; the token now includes workflow scope.
Moved the workflow to `.github/workflows/ci.yml` and updated current setup guidance.
The initial upload failure above is retained as historical evidence.

First remote CI run passed for commit `ea2c7cb`: formatting, Clippy, all five Rust
tests, optimized build and all three PTY exit/recovery checks.
[GitHub Actions evidence](https://github.com/gigpies/shr-lux/actions/runs/36486029648).
The same normal suite passed locally before pushing. Physical tests remain pending;
no historical/exhaustive suite exists. GitHub reported a checkout@v4 Node runtime
deprecation warning; it did not fail the run.
