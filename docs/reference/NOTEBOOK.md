# shr-lux project notebook

A standalone zk notebook of linked Markdown notes. Keep observed facts, protocol
references, proposals, decisions and test results distinguishable. Sources were consulted
on 2026-09-28 unless another date is stated. The original [idea](../../idea.md) remains intact.

For GigPies module work, plan and track implementation only in
[the owning plan](../notes/0027-gigpies-implementation.md). The dated notes below
are technical references/evidence, not competing task queues.

## Notes

- [Project map](../notes/0001-project-map.md)
- [Hardware baseline — 2026-09-28](../notes/0002-hardware-baseline.md)
- [uDMX USB protocol](../notes/0003-udmx-protocol.md)
- [Linux USB access](../notes/0004-linux-usb-access.md)
- [Architecture and state ownership](../notes/0005-architecture.md)
- [DMX and fixture patching](../notes/0006-dmx-fixtures.md)
- [Terminal interaction contract](../notes/0007-terminal-contract.md)
- [Audio inputs and musical estimates](../notes/0008-audio-and-musical-state.md)
- [MIDI control and feedback](../notes/0009-midi-feedback.md)
- [Development milestones](../notes/0010-roadmap.md)
- [First fixture bench test](../notes/0011-fixture-bench-test.md)
- [Development and validation](../notes/0012-development-and-validation.md)

- [Scaffold validation](../notes/0013-scaffold-validation.md)

- [Pi 1 / 512 MB feasibility and deployment](../notes/0014-pi1-feasibility.md)
- [Musical understanding and punk/metal programs](../notes/0015-musical-direction.md)
- [Show-policy prototype and validation](../notes/0016-show-policy-validation.md)

- [Local four-AUX simulation: punk and metal](../notes/0017-local-aux-simulation.md)

- [Music analysis and MiniLab pad preview](../notes/0018-analysis-and-pad-preview.md)

- [Composed eight-fixture pad show](../notes/0019-composed-pad-show.md)

## Lighting design study — 2026-09-29

- [Study map and conclusions](../notes/0020-lighting-design-study.md)
- [Composition, visibility and attention](../notes/0021-composition-and-attention.md)
- [Color theory and real fixtures](../notes/0022-color-and-fixtures.md)
- [Musical time, chases and storms](../notes/0023-musical-time-and-motion.md)
- [Design specification and implementation priorities](../notes/0024-design-to-engine.md)
- [Audition laboratory](../notes/0025-design-audition-lab.md)
- [Annotated primary sources and further reading](../notes/0026-lighting-study-sources.md)

## Using zk

From the project root:

```sh
zk --notebook-dir docs index
zk --notebook-dir docs list
zk --notebook-dir docs list --match udmx
zk --notebook-dir docs new docs/notes --title "Fixture test observations"
```

Or `cd docs` and use zk normally. This nested notebook is deliberately independent of
any parent notebook. Regular `.md` links also work on GitHub and without zk installed.
The generated index database is ignored; notes, config and template are versioned.
New notes should link to related notes and be added here. Never turn an untested
assumption into a hardware fact. Keep large recordings and private venue details in
ignored local storage; add only the small evidence needed to explain a result.

zk may warn that links to the root idea.md are outside this notebook; those links
are intentional and resolve normally in Markdown.

## GigPies integration — 2026-10-04

[Owning GigPies plan](../notes/0027-gigpies-implementation.md) records scoped tasks, contract dependencies,
validation and historical launch instructions. LX-01–04 are accepted software;
LX-05 is implemented and device-free software validation has passed; root runtime
acceptance remains separate. Physical output remains
unverified and disarmed.

[Null-output static authority](../notes/0028-static-authority.md) documents the
manual command API, copies/masks, provenance and preview-only release boundary.

- [0029 C-LIGHT encoded local authority](../notes/0029-lighting-wire.md)

- [0030 Timed release and private restart](../notes/0030-release-recovery.md)

- [0031 Real private provider integration](../notes/0031-local-provider-integration.md)

- [0032 Independent named analysis subscription](../notes/0032-analysis-subscription.md)
- [0033 Configured synthetic authority and named analysis](../notes/0033-configured-analysis.md)
- [0034 Bounded harmonic and arrangement estimates](../notes/0034-musical-analysis.md)
- [0035 Authored musical scene director](../notes/0035-scene-map.md)

## CI incident notifications

[CI failure and recovery](../CI.md#failure-and-recovery-notifications) describes the
GitHub issue thread that tracks each current default-branch failure and recovery.
This is repository automation; it does not activate lighting or other hardware.
