# Project map

Date: 2026-09-28

Status: project direction; implementation scope explicitly limited.

The authoritative original concept is [idea.md](../../idea.md), preserved unchanged.
The product is an autonomous lighting runner for small live bands: separate instrument
feeds, known performer positions, a venue fixture patch, and manual MIDI control with
LED feedback. Rust on Raspberry Pi; target display 80×25.

## Implemented now

- Rust application and keyboard-driven TUI shell.
- Read-only USB descriptor diagnostics for the observed uDMX identity.
- Validated DMX addresses and a pure uDMX range-request encoder, tested without output.
- This notebook, project working agreements, local checks and GitHub CI.

Audio capture, musical analysis, fixture patch loading, output, and MIDI are not implemented.
Unknown measurements remain unavailable in the UI; no simulated BPM or energy is shown.

## Navigation

- [Architecture](0005-architecture.md)
- [First milestones](0010-roadmap.md)
- [Hardware evidence](0002-hardware-baseline.md)
- [Terminal behavior](0007-terminal-contract.md)

[Notebook index](../reference/NOTEBOOK.md)
