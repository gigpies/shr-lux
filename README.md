# SHR Lux

**The lighting engine behind GigPies.** Build scenes around instrument activity
and musical changes, while keeping manual control through SHR Lightdesk.

[**Try a show preview**](docs/USING.md) · [**Lighting desk**](https://github.com/gigpies/shr-lightdesk) · [Engineering](docs/ENGINEERING.md)

## What it does today

- Runs manual looks, cues, Hold and reviewed transitions for Lightdesk.
- Supports authored scene maps and limited, explicitly enabled automation.
- Estimates musical activity; the integrated analyzer can also estimate harmony
  and recurring sections.
- Previews recorded material in a terminal or, optionally, on MiniLab mkII pads.

**Physical DMX output and standalone live audio capture are unfinished.**
Integrated scenes currently produce software state only. Harmony and section
labels are estimates; beat/downbeat recognition is not available in the
integrated analyzer.

## Preview without equipment

From this checkout, with its pinned Rust toolchain:

```sh
cargo run --locked --example show-preview -- punk
```

This prints a synthetic show timeline. It does not listen to audio or control
lights. For recorded music and optional pad previews, use the
[preview guide](docs/USING.md).

[Build requirements](docs/USING.md#build) · [Research and technical notebook](docs/reference/NOTEBOOK.md) · [Original concept](idea.md)

No project license has been selected. External recording terms are linked from
the [simulation guide](docs/notes/0017-local-aux-simulation.md).
