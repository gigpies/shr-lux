# uDMX USB protocol

Date: 2026-09-28

Status: upstream source reference; vendor writes on our dongle still untested.
Tags: #dmx #usb #reference

uDMX is a USB protocol and dongle firmware that produces DMX. It is not an FTDI
Open DMX serial adapter. Do not try to drive this unit as `/dev/ttyUSB0`.

## Identity and transport

Our device advertises 16c0:05dc plus www.anyma.ch / uDMX. The numeric pair is shared;
check strings before offering an output target. Fail on ambiguous device selection.
The upstream master inspected uses PID 05e4 for its MIDI variant. Do not replace our
observed PID with that constant blindly.

USB control requests go to endpoint 0. A prospective Rust implementation can use
rusb/libusb-1.0. Old Anyma Max external instructions about libusb-0.1 describe that
software's dependency; they are not a requirement to copy its host implementation.

## Request reference

| Field | Set single channel | Set channel range |
|---|---|---|
| bmRequestType | 0x40 (vendor, device, OUT) | 0x40 |
| bRequest | 1 | 2 |
| wValue | level, 0–255 | number of slots |
| wIndex | slot index, 0–511 | first slot index, 0–511 |
| Payload | see upstream host/firmware before implementing | contiguous slot bytes |
| Length | upstream header says ignored | use exactly slot count |

Range count must be 1..512 and start + count must be <=512. User-facing DMX
addresses are 1..512: address 1 becomes USB index 0. Do not prefix a DMX start code
to the range payload. The firmware handles serial framing.

The scaffold implements only pure range encoding; it does not submit the request.
For future transfers, check returned byte count and USB errors. USB success proves
transfer completion, not light output. Use bounded timeouts and expose failures.

Request 0xf8 starts the bootloader. It has no place in normal diagnostics or show code.
Upstream code starts DMX output when channel data is received. Disconnect behavior
and retained values must be tested on our clone; never equate closing USB with blackout.

## Sources

Inspected 2026-09-28, upstream commit `948dba1e2cc93cfbbef6c36601b4f2c90764634b`:

- [Anyma project](https://www.anyma.ch/research/udmx/)
- [Command definitions](https://github.com/mirdej/udmx/blob/948dba1e2cc93cfbbef6c36601b4f2c90764634b/common/uDMX_cmds.h)
- [Firmware request handling](https://github.com/mirdej/udmx/blob/948dba1e2cc93cfbbef6c36601b4f2c90764634b/firmware/main.c)
- [Host utility](https://github.com/mirdej/udmx/blob/948dba1e2cc93cfbbef6c36601b4f2c90764634b/commandline/uDMX.c)
- [rusb device handle](https://docs.rs/rusb/0.9.4/rusb/struct.DeviceHandle.html)

Upstream firmware/software is GPL-2.0-or-later per Anyma. We have not vendored its
implementation. Review licensing before copying upstream source; this private project
has no chosen distribution license yet.

Related: [DMX fundamentals](0006-dmx-fixtures.md), [hardware](0002-hardware-baseline.md).

[Notebook index](../reference/NOTEBOOK.md)
