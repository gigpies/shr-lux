# Composition, visibility and attention

Date: 2026-09-29

Status: sourced fundamentals followed by proposed applications. Tags: #design #composition

## Begin with the audience's view

ETC's introduction distinguishes visibility, revelation of form, composition,
mood and information. It treats intensity, color, distribution and movement as
controllable means toward those objectives. That is a better starting question
than “which effect comes next?”
[Objectives](https://blog.etcconnect.com/stage-lighting-design-part-2-objectives-of-lighting-design),
[controllable properties](https://blog.etcconnect.com/stage-lighting-design-part-3).

For each proposed look, finish this sentence: **“At this moment I want the audience
to notice …”** Examples for our project: the whole band playing together, the
left guitarist's exposed entry, the weight of a slow riff, or the empty space
before a return. If every answer is “the lights,” revise the look.

This is an artistic heuristic. Deliberately foregrounding the lighting can also
be appropriate, especially during an instrumental interlude. The system needs
an explicit reason for doing so.

## Angle creates form; color cannot substitute for it

Front, side, back and top light reveal different aspects of a performer. ETC's
angle guide explains how front light supports faces, side light shapes bodies,
back light separates a figure from its surroundings, and top light can conceal
facial detail. Its familiar 45-degree arrangements are teaching examples, not a
rigging prescription for an unknown venue.
[Lighting angles](https://blog.etcconnect.com/stage-lighting-design-part-5).

**Proposed stage roles:**

| Role | First job | Allowed development | Failure to avoid |
| --- | --- | --- | --- |
| Performer/front | Keep intended people readable | Gentle balance and focus shifts | Chase repeatedly removes faces |
| Rear/side environment | Establish depth and palette | Color movement, silhouettes, broad changes | Saturated rear field overwhelms the subject |
| Accent | Give selected moments punctuation | Hits, brief chases, occasional storm | Every transient becomes an event |
| Scenic/background | Define the space | Slow gradients, negative space | All surfaces equally active |

These are logical roles, not extra fixture requirements. A four-fixture rig may
have to prioritize coverage over effects. With eight pads, we can draw roles but
cannot prove coverage. With borrowed lamps we must learn their actual positions,
beams and useful levels before assigning responsibilities.

## Five compositional decisions to make explicitly

1. **Focal hierarchy.** Which area is primary, which supports it, and which recedes?
   A guitarist becoming louder may justify a cautious shift of emphasis; it does
   not identify who is singing or prove a solo. Keep performer positions configured.
2. **Occupied space.** Does the picture feel compact, wide, low, tall, centered or
   divided? Specify this before choosing a chase direction.
3. **Balance.** Symmetry can make a unified, stable picture; asymmetry can give a
   passage direction. These are expressive options to audition, not emotional laws.
4. **Negative space.** Which fixtures or areas are intentionally quiet? Do not
   confuse “unused” with “missing.” Keeping every pad bright makes absence unavailable.
5. **Contrast and reserve.** What can the next important moment change? If the
   whole stage is already bright, saturated, wide and busy, an increase has few places to go.

The questions above are our practical design rubric. They are not a formula that
assigns numerical beauty scores to a stage.

## Lessons from three firsthand practices

**Andi Watson / Mitski:** Watson describes shaping environments around the artist
and choreography, accounting for sightlines, and returning to stillness after a
transformation. The transferable lesson is to make room for the performer and
think about the whole audience. The specific choreography and touring apparatus
are not features our AUX feeds can infer.
[Watson interview](https://www.wallpaper.com/art/music/mitski-the-land-lighting-andi-watson).

**Rob Koenig / Metallica:** Koenig explains the importance of picking the band out
of a very large visual picture and of audience connection. This is useful evidence
against equating metal with obscuring the musicians under constant effects.
[Koenig interview](https://chauvetprofessional.com/news/im-with-the-band-rob-koenig-and-metallica/).

**Rob Sinclair / Queen Extravaganza:** Sinclair describes intentionally revealing
more of the rig as the show develops and using strong blocks of single color.
Our inference is that a small rig can also reserve a spatial grouping or a new
contrast for later. We do not need to imitate an arena rig to use that principle.
[Sinclair interview](https://www.livedesignonline.com/business-people-news/rob-sinclair-lighting-queen-extravaganza-tour).

These examples do not imply one mandatory style. They show designers making
choices about people, space and development before multiplying effects.

## A concrete eight-fixture exercise

**Proposal, not a physical patch:** imagine four rear fixtures ordered left to
right and four performer fixtures covering the band. Start with stable performer
coverage and one restrained rear color. Make three variants using only the rear
layer:

- A centered pool with the edges quiet.
- A wide picture with a quiet center.
- A left-to-right gradient with no motion.

Keep palette and approximate total emitted level comparable. Ask which picture
best fits a sustained riff. Only then add one slow moving element. If motion adds
nothing, keep the still variant. Repeat on pads using an explicit role legend;
then repeat on actual fixtures when a known patch exists.

A fair comparison must not make the “more complex” version dramatically brighter.
Our controller cannot equalize perceived brightness, so record that limitation
rather than claiming a controlled photometric experiment.

## Current implementation audit

`src/preview.rs` treats all eight pads as interchangeable members of one row. It
has no performer-light role, no depth coordinate and no protected coverage layer.
Its deliberate mirror geometry is useful for studying order but cannot establish
that a band would be visible. The new storm demonstrated that a different spatial
organization can communicate a different effect without adding colors.

**Next design step:** a small explicit layout and a scene description that says
what is held, what is moving, what remains dark and which area carries the accent.
Do not add moving-head control merely to compensate for a missing composition.

Related: [study map](0020-lighting-design-study.md), [color](0022-color-and-fixtures.md),
[engine proposals](0024-design-to-engine.md).

[Notebook index](../reference/NOTEBOOK.md)
