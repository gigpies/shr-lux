# Preview a lighting show

[Project home](../README.md) · [Lightdesk controls](https://github.com/gigpies/shr-lightdesk/blob/main/docs/USING.md) · [Engineering](ENGINEERING.md)

Lux can preview recorded music in the terminal and on MiniLab mkII pads.
**These previews do not drive DMX fixtures.** In GigPies, operate Lux through
Lightdesk; the integrated service and standalone replay have different analysis
capabilities.

## Build

Use the checkout's pinned Rust toolchain and a C compiler. On Linux, install
`libasound2-dev` and `pkg-config`. Optional listening uses SoX `play`.

```sh
cargo build --locked --release
```

## Try a program without recordings

```sh
cargo run --locked --example show-preview -- punk
cargo run --locked --example show-preview -- metal --bursts
cargo run --locked --example show-preview -- atmospheric
```

These print synthetic timelines without waiting, playing sound or touching
hardware. They show scene changes and accents, not live musical recognition.

## Preview recorded music

Prepare local material using the [recording preparation guide](notes/0017-local-aux-simulation.md).
Its examples use separate kick, bass and two guitar tracks from punk and metal
sessions. The recordings remain local and have their own source terms.

```sh
target/release/shr-lux simulate punk
target/release/shr-lux simulate metal
```

The terminal shows a coordinated eight-fixture preview. **q/Esc/Ctrl+C** quits,
**b** toggles burst requests and **x** toggles blackout.

To enable connected MiniLab mkII pad output and audible monitoring explicitly:

```sh
target/release/shr-lux simulate metal --midi --listen --bursts
```

Omit `--midi` for terminal-only output and `--listen` for silence. Listening and
analysis run on separate clocks, so this is an approximate audition. Pad colours
are stepped; they cannot demonstrate smooth fixture fades.

## Music-following in GigPies

The configured service supports authored scenes and bounded intensity/colour
automation. Lightdesk previews changes and requests control explicitly.
The integrated analyzer can estimate harmony and recurring sections, but
beat/downbeat remain unavailable. Standalone replay does not provide those same
harmony/section estimates. [Scene controls](https://github.com/gigpies/shr-lightdesk/blob/main/docs/USING.md#scenes-and-music-following).

For current output limits, service setup and protocol details, use
[Engineering](ENGINEERING.md). The [lighting design study](notes/0020-lighting-design-study.md)
is optional background, not required reading to run a preview.
