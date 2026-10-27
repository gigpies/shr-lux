# Development milestones

Date: 2026-09-28

Status: proposed sequence. Tags: #planning

## Revised priority — 2026-09-29

Target Pi 1 / 512 MB and selectable punk/metal programs. A pure policy prototype
and synthetic preview now precede hardware output. Recorded level/kick analysis and MiniLab pad previews are now implemented.
Next: label real kick onsets to measure accuracy, add live capture, and measure ARMv6
deployment. See [current evidence](0018-analysis-and-pad-preview.md). Develop the fixture
path below as a separate gate; keep output disabled until its requirements pass.
See [feasibility](0014-pi1-feasibility.md), [research and work sequence](0015-musical-direction.md)
and [implemented scope](0016-show-policy-validation.md).

## Design learning track — 2026-09-29

The [lighting design study](0020-lighting-design-study.md) adds a proposed sequence:
repeatable auditions → defined color model → hold-first direction → role-aware
composition → bounded storm integration. This is research guidance, not implemented
runtime behavior. It complements capture and Pi 1 work. The user approved the composed
pad show and scattered storm texture; physical fixtures still need separate validation.

## 0 — Foundation (this scaffold)

Runnable Rust TUI shell, descriptor-only doctor, pure DMX address/range contracts,
linked notebook, working agreements, normal tests and CI. No lighting output.

## 1 — One real fixture

Record fixture model/mode/address, install the scoped USB permission rule, implement
bounded uDMX transport, explicit output arming and fixture-aware blackout. Verify one
known dimmer/color at low level, then confirm disconnect/reconnect and exit behavior.
Record observations in a dated note. See [bench checklist](0011-fixture-bench-test.md).

## 2 — Fixture abstraction and manual operation

Versioned venue patch, overlap/range validation, capability mapping, steady scenes,
master level, blackout latch. Define ownership and persistence; then add MIDI learn
and actual controller feedback once hardware is identified.

## 3 — Musical response

Capture and replay source feeds. Prove activity/transient detection, then add rhythm
and confidence-driven scheduling. Hold looks through uncertain input; enforce dwell
and transition rules to avoid flickering decisions.

## 4 — Richer understanding

Evaluate bars, section changes, harmony and performer focus on actual band material.
Measure false transitions as well as detection accuracy. Keep manual correction usable.

## Open hardware facts

Exact borrowed cans and channel maps; MiniLab brightness capabilities; audio interface models; available
mixer outputs; cable/termination arrangement; measured USB/DMX latency and output behavior.

Related: [project map](0001-project-map.md), [original idea](../../idea.md).

[Notebook index](../reference/NOTEBOOK.md)
