# DMX and fixture patching

Date: 2026-09-28

Status: reference plus proposed project conventions. Tags: #dmx #fixtures

DMX512 carries up to 512 data slots per universe. Each slot holds an 8-bit value.
Normal lighting packets use start code zero. The serial link uses 250 kbit/s, 8 data
bits and two stop bits. A break and mark-after-break delimit packets; the dongle
firmware, not the TUI scheduler, generates that electrical timing.

A channel is a slot, not necessarily a fixture. A fixture's selected channel mode
assigns meanings to consecutive slots: dimmer, colors, shutter, strobe, movement,
macros, and so on. Those meanings require the actual fixture manual.

## Patch conventions

- Display addresses 1..512; convert to zero-based indexing only at the transport boundary.
- Check that start address + footprint - 1 <=512.
- Reject overlapping fixture ranges by default; intentional mirroring needs an explicit design.
- Record make/model, mode, address, group and performer association.
- Model capabilities: RGB-only PARs cannot implement pan/tilt or arbitrary color temperature.
- A mover target is a calibrated preset, not inferred physical tracking.
- Distinguish shutter-open and strobe-disabled values; never guess from a generic PAR layout.

For initial testing, identify fixture mode and cable pinout, daisy-chain appropriately,
and terminate the final DMX input chain with the correct terminator. Do not assume
3-pin and 5-pin connectors have interchangeable wiring without checking adapters.
Our dongle's isolation and electrical quality have not been established.

## Sources

- [Open Lighting DMX512 reference](https://wiki.openlighting.org/index.php/DMX512)
- [ESTA DMX512-A guide](https://tsp.esta.org/tsp/documents/docs/DMX512-A_Guide_%288x10%29_ESTA.PDF)
- [ESTA start-code registry](https://tsp.esta.org/tsp/working_groups/CP/DMXAlternateCodes.php)

These notes are implementation guidance, not a claim of standards compliance.
Related: [uDMX](0003-udmx-protocol.md), [bench test](0011-fixture-bench-test.md).

[Notebook index](../reference/NOTEBOOK.md)
