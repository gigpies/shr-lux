# First fixture bench test

Date: 2026-09-28

Status: pending borrowed lights; no physical output test performed. Tags: #hardware #testing

1. Record fixture manufacturer, model, chosen DMX mode and start address. Photograph
   or transcribe the relevant channel table into a local reference note.
2. Confirm the cable/adapter wiring and termination. Connect one fixture first.
3. Apply [USB permissions](0004-linux-usb-access.md) and verify `doctor` as the normal user.
4. Implement/review the output path against the [protocol reference](0003-udmx-protocol.md).
   The current scaffold has no send command.
5. Set a known non-strobing static mode. Start with a low known dimmer level and one
   color according to the actual channel map. Avoid sweeps across unknown function slots.
6. Check address conversion at the first and last slots of the fixture footprint.
7. Confirm blackout, manual level changes, output timeout, clean exit, and reconnect.
   Note whether the fixture holds its last look when USB or DMX disappears.
8. Add the other cans after the first fixture behaves predictably.

Record software commit, fixture configuration, commands, observed response and failures.
USB transfer success and visible fixture response are separate evidence. A fixture test
also does not certify full DMX electrical timing; a tester/oscilloscope would be needed.

Do not enable automatic output simply because the dongle reconnects. The later show
engine must specify how the operator resumes after output loss.

Related: [hardware baseline](0002-hardware-baseline.md), [DMX patching](0006-dmx-fixtures.md).

[Notebook index](../reference/NOTEBOOK.md)
