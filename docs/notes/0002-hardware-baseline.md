# Hardware baseline — 2026-09-28

Date: 2026-09-28

Status: observed on the actual host. Tags: #hardware #evidence

| Item | Observation |
|---|---|
| Host | Raspberry Pi 5 Model B Rev 1.1, AArch64 |
| OS | Debian GNU/Linux 13 (trixie), VERSION 13.6 |
| USB identity | 16c0:05dc; manufacturer www.anyma.ch; product uDMX |
| Serial descriptor | ilLUTZminator001 |
| USB device revision | bcdDevice 1.02; not proof of a particular firmware source revision |
| Enumeration | Bus 003, device 002 at inspection; these numbers can change |
| Interface | Vendor device, interface 0, no extra endpoints; control transfers use endpoint 0 |
| Kernel interface driver | None bound; userspace USB access is expected |
| Permissions | root:root, mode 0664; shome cannot open device with libusb |
| Root probe | sudo lsusb -v succeeded, including Device Status 0x0000 |
| Other USB peripherals | No audio interfaces or MIDI controller appeared in lsusb |

The device enumerates and responds to standard USB queries. Neither vendor DMX writes,
the RS-485 transmitter, cable, nor fixture behavior has been validated. No DMX values
were sent. User plans to borrow PAR cans for a later bench test.

The advertised 500 mA is a USB descriptor field, not a measured power draw.
Shared VID/PID and serial strings do not establish physical manufacturer, isolation,
or exact firmware. Do not flash firmware based on this evidence.

Evidence commands: `lsusb`, `lsusb -t`, `lsusb -v -d 16c0:05dc`,
`sudo lsusb -v -d 16c0:05dc`, `udevadm info`, device-node permissions,
`/proc/device-tree/model`, `/etc/os-release`.

Related: [USB access](0004-linux-usb-access.md), [uDMX protocol](0003-udmx-protocol.md),
[fixture bench test](0011-fixture-bench-test.md).

[Notebook index](../reference/NOTEBOOK.md)
