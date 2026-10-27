# Linux USB access

Date: 2026-09-28

Status: rule prepared in repository; not installed on the host.
Tags: #hardware #operations

The initial device node was root:root 0664. `shome` belongs to `plugdev`, but that does
not help until the device is assigned that group with write access. libusb opens USB
nodes for read/write even for descriptor diagnostics.

The proposed rule matches VID/PID and manufacturer/product, then assigns plugdev and
0660. It avoids granting every local user access to every device sharing this USB ID.

## Install when preparing fixture tests

Run from the repository root:

```sh
sudo install -m 0644 contrib/udev/70-shr-lux-udmx.rules /etc/udev/rules.d/70-shr-lux-udmx.rules
sudo udevadm control --reload-rules
```

Unplug/replug the dongle, then run:

```sh
lsusb -d 16c0:05dc
cargo run --locked -- doctor
```

The diagnostic should read the identity without sudo. Recheck the current USB device
node if it still reports access denied. Bus/device numbers change after reconnect.
On another machine, ensure the intended user belongs to plugdev and has logged in again.

To undo, remove only `/etc/udev/rules.d/70-shr-lux-udmx.rules`, reload rules, and reconnect.
Do not run the full app as root to work around permanent permissions.

Reference: [OLA device setup](https://www.openlighting.org/ola/getting-started/device-specific-configuration/).
Related: [observed hardware](0002-hardware-baseline.md), [bench procedure](0011-fixture-bench-test.md).

[Notebook index](../reference/NOTEBOOK.md)
