# MIDI control and feedback

Date: 2026-09-28

Status: MiniLab mkII identified; opt-in pad feedback implemented. Tags: #midi #controller

Observed 2026-09-29: Arturia MiniLab mkII, USB `1c75:0289`, ALSA `hw:2,0,0`.
The opt-in preview writes researched pad-color SysEx on both logical banks. It never
stores presets or changes mappings. Physical color confirmation is still pending.
See [protocol sources and test evidence](0018-analysis-and-pad-preview.md).

Input mappings should produce semantic actions. Feedback derives from resolved active
state so automatic changes, TUI actions and MIDI actions agree. Avoid MIDI feedback loops:
a sent LED update must not be interpreted as another user action.

Define whether each control is momentary, latched, absolute, or relative. Non-motorized
faders need a deliberate pickup/takeover strategy. No touch sensor is assumed.
Blackout stays latched until explicit release; an LED should distinguish it from an
unplugged or unknown controller state.

Current output uses Linux ALSA raw MIDI in nonblocking mode on a bounded worker.
MIDI input actions and takeover remain proposals; no input mapping is implemented.

Related: [architecture](0005-architecture.md), [roadmap](0010-roadmap.md).

[Notebook index](../reference/NOTEBOOK.md)
