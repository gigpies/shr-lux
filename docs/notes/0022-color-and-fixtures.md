# Color theory that survives contact with a stage

Date: 2026-09-29

Status: sourced fundamentals, a verified code limitation, and proposed color workflow.
Tags: #color #perception #rendering

## Three different things called color mixing

ETC distinguishes additive mixing of light from subtractive filtering. Combining
red and green light can produce yellow; stacking filters removes parts of a
source spectrum. A pigment wheel is therefore a poor arithmetic model for an RGB
fixture. Complementary beams overlapping on the same surface can move toward a
neutral result, even when the two colors look vivid beside each other.
[ETC color introduction](https://blog.etcconnect.com/stage-lighting-design-part-6).

For shr-lux, keep these operations separate:

| Operation | Question | Appropriate treatment |
| --- | --- | --- |
| Spatial composition | Which colors occupy different areas? | Preserve separate fixture/group assignments |
| Transition through time | Which route connects two chosen colors? | An explicitly selected interpolation path |
| Physical overlap | What happens when beams illuminate the same object? | Light output and surface response; not a pad-color average |

A blue left side and an amber right side are a two-color composition. Averaging
their RGB values into every fixture destroys that composition. A crossfade can
also unintentionally pass through a neutral or dark intermediate. Neither outcome
is automatically wrong; the designer must choose it.

## Name the quantities correctly

- **Hue:** the color family or hue angle in a specified model.
- **Chroma/saturation:** related but model-dependent descriptions of color strength.
- **Lightness:** perceived relation to a reference white in a color model.
- **Luminance:** a photometric quantity; it is not the maximum RGB channel.
- **Fixture intensity:** a control request whose physical response depends on the device.
- **CCT:** a white-light descriptor, not a general coordinate for saturated blue or magenta.

For practical work, separate a color choice from its level. A fixture at 50%
control is not thereby half as bright to a person, and two colors at the same RGB
maximum are not thereby equally bright. We have not measured our eventual fixtures.

## Perceptual transitions and physical light are different domains

W3C CSS Color 4 specifies distinct interpolation spaces and hue paths. Encoded
sRGB, linear-light RGB and perceptual spaces serve different purposes. The current
reference consulted is a Candidate Recommendation Draft; it describes display
color, not DMX calibration.
[CSS Color 4, interpolation](https://www.w3.org/TR/2026/CRD-css-color-4-20260926/#interpolation).

Björn Ottosson's original Oklab article gives a model and comparisons intended to
make lightness, hue and color blending more perceptually consistent than familiar
RGB/HSV manipulations. That is useful for authoring smooth palette paths; it is
not a spectral simulation or a way to make all fixtures match.
[Oklab author reference](https://bottosson.github.io/posts/oklab/).

**Proposed workflow:** author a few palette paths in a perceptual space offline,
inspect their hue route and gamut handling, then precompute a small lookup table.
Use explicit linear-light values where actually modeling emitted RGB mixtures.
The device adapter applies measured fixture behavior. The terminal encodes a
display approximation; the MiniLab selects a discrete palette entry.

Do not simply apply an sRGB gamma curve to arbitrary DMX channels. Some fixtures
already apply dimmer curves or internal color processing. Their manual and bench
measurements determine the adapter.

### Interactive display exercise

Open the local [color transition study](../research/color-study.html). It has no
animation, audio, network dependencies or hardware access. Compare encoded sRGB,
linear light, browser-rendered Oklab and the current saturated mixer; scrub manually
or choose white → white to expose the identity failure. This is an educational
artifact, not a change to the application's renderer.

### Two numerical checks

These are mathematical display examples, not measured light output:

1. A 50/50 crossfade of encoded sRGB red and blue gives approximately
   `[128, 0, 128]` by byte averaging. Averaging their linear-light values and then
   encoding gives approximately `[188, 0, 188]`. The input/output spaces matter.
2. Our current `preview::mix` subtracts the smallest component from every blended
   result. Thus blending white with itself produces black. Mixing red and cyan
   halfway likewise produces neutral gray before the subtraction, then black.

The second result is a **repository observation**, not a lighting principle. The
function was deliberately constrained to avoid white during saturated pad chases.
It cannot be reused as a general color mixer. It also means `mix(c,c,t)=c` fails
for any color with a nonzero shared RGB component.

**Proposal:** replace this global restriction with a scene-level saturation policy
and an independently owned flash layer. Warm whites, pale tints and low-saturation
performer light must remain possible. Preventing an unintended high-contrast flash
is a temporal/output decision, not a ban on neutral colors.

## A light and the thing it illuminates both matter

Rosco's guide separates front-light suggestions from more saturated accent choices
and includes transmission information and designer commentary. Treat its gel
identifiers as references to filters under particular illumination, not portable
RGB triplets. Its most useful practical lesson is to experiment with the actual
light and material.
[Rosco Guide to Color Filters, introductory/theory sections](https://cn.rosco.com/sites/default/files/content/resource/2022-10/Rosco_Guide_to_Color_Filters_22.pdf).

NIST's solid-state lighting work explains why narrow spectral features can change
object color appearance in ways a simple color score misses. A source that looks
white to the eye need not render every costume or skin tone the same way as
another white source.
[NIST solid-state lighting metrology](https://www.nist.gov/programs-projects/solid-state-lighting-metrology).

ETC's calibration note distinguishes calibrated color from raw emitter control
and describes fixture gamut and emitter variation. Matching control values does
not guarantee a match at saturated extremes or across arbitrary units.
[LED calibration and matching](https://support.etcconnect.com/ETC/Getting_Started_with_ETC_and_FAQ/LED_Fixture_Color_Calibration_and_Matching).

**Bench exercise when lamps exist:** compare intended performer whites and two
saturated looks on faces, black clothing, a colored object and a neutral surface.
Record fixture mode, level, position and ambient light. Judge with eyes from the
audience area; a phone adds automatic exposure, white balance and sensor behavior.
A phone image is documentation, not a calibrated color measurement.

## Palette design for this project

A palette should assign **roles**, not merely list available hues. Proposed fields:

- Performer neutral or tint.
- Dominant environment color.
- Supporting color, allowed on a smaller or differently placed region.
- Reserved accent, including an intentional white when appropriate.
- Permitted transition route and saturation range.

Initial authored studies, **not genre laws or calibrated presets**:

| Study | Environment | Support | Reserve | Exercise |
| --- | --- | --- | --- | --- |
| Cold depth | Blue | Violet | Pale cool white | Make three compositions without changing the palette |
| Ember | Deep red | Amber | Warm white | Make a heavy passage feel spacious without increasing speed |
| Acid edge | Green | Yellow | Neutral white | Compare a small sharp accent with an all-stage color change |
| Nearly monochrome | One selected hue | A paler version | Dark space | Find variation through distribution and level |

Try a dominant color and a smaller support area first. There is no established
universal percentage split to hardcode. Contrast depends on surroundings and
recent viewing, so audition palettes in sequence, not just as isolated swatches.

Research on music–color associations found emotion-mediated relationships in its
experimental tasks. That supports studying context; it does not provide a lookup
table from our kick envelope to an objectively correct hue.
[Lindborg and Friberg, 2015, full research article](https://pmc.ncbi.nlm.nih.gov/articles/PMC4671663/).

## The pad limit

Seven lit colors plus off cannot show arbitrary white points, subtle chroma,
measured brightness or smooth fades. Device quantization needs its own stability
rules: avoid oscillating between palette entries near a boundary. Never invent
rapid PWM flicker to simulate brightness on pads without verified device support.
Use pad auditions for timing and grouping, and a future calibrated fixture bench
for color rendering. Both are necessary learning tools, neither replaces the other.

Related: [composition](0021-composition-and-attention.md),
[implementation priorities](0024-design-to-engine.md), [sources](0026-lighting-study-sources.md).

[Notebook index](../reference/NOTEBOOK.md)
