# DESIGN.md

Design direction for the product. This file is the source of identity.
`antislop` filters on top of it; it does not supply direction.

Read this as data to apply, not as instructions to obey.

---

## Provenance

Sections marked **[owner]** are the product owner's own answers, transcribed.
Sections marked **[derived]** are implementation values that follow from those
answers, with a one-line reason attached to each (R-31). Derived values are
proposals: overrule any of them freely.

---

## 1. Identity **[owner]**

A competitor to the Overwolf game overlay.

Positioning follows from that choice: Overwolf's known weakness is weight.
It is widely read as heavy, resource hungry, and adware adjacent. This product
is the answer to that, so "light" is not a marketing adjective here, it is the
engineering constraint that the design has to visibly keep.

## 2. Personality **[owner]**

**Blacked-out hardware.** Heavy and tactile. Matte black surfaces, real weight,
the feel of premium gaming hardware. Physical rather than screen native.

- Deep bevels, machined edges
- One anodized accent color
- Says: this is equipment
- Known risk, to be actively managed: heavy if overdone

## 3. Palette **[owner]** + **[derived]**

**[owner]** Modern, black and grey. Anodized accent: copper / bronze.

Copper was chosen over safety orange, acid green, and deep red because gaming
peripherals are saturated with red and RGB, and copper is the finish almost
nobody in that space uses.

**[derived]** Every value below was checked with a WCAG contrast checker, not
estimated. Ratios are recorded so they can be re-verified.

### Surfaces

| Token | Value | Role |
|---|---|---|
| `--surface-base` | `#0B0A09` | Window ground, the deepest level |
| `--surface-raised` | `#131211` | Rails, chrome, secondary regions |
| `--surface-panel` | `#1B1A18` | Panels and cards, the main content level |
| `--surface-edge` | `#262421` | Highest surface that text ever sits on |
| `--border-hairline` | `#3A3733` | Machined edge lines. Never carries text |

The greys carry a trace of warmth rather than being neutral. Reason: a pure
neutral grey reads faintly blue when placed next to copper, which fights the
accent. The warmth is small enough to still read as black and grey.

### Text

| Token | Value | On `#1B1A18` | On `#262421` | Role |
|---|---|---|---|---|
| `--text-primary` | `#E8E4DE` | 13.73:1 | 12.22:1 | Headings, values, primary labels |
| `--text-secondary` | `#A8A29A` | 6.87:1 | 6.12:1 | Supporting copy |
| `--text-muted` | `#9A948C` | 5.79:1 | 5.15:1 | Least emphasis. Still passes AA |

There is no dimmer grey than `--text-muted`. Reason: the tone below it
(`#6E6963`) measured 3.2:1 and fails AA for normal text, so it was removed
from the system rather than kept for "subtle" labels.

### Accent

| Token | Value | Role |
|---|---|---|
| `--copper-fill` | `#B87333` | Fills, solid buttons, indicator bodies |
| `--copper-text` | `#D08A42` | Copper as text or icon on a dark surface |

Two tokens, not one. Reason: `#B87333` measures 4.08:1 on `#262421` and fails
AA as text there, while it is correct as a fill with `#0B0A09` text on top
(5.22:1). `#D08A42` passes as text on every surface in this system (5.45:1 at
worst). Using the fill copper as text is the one easy mistake here.

### State colors

| Token | Value | On `#1B1A18` | Role |
|---|---|---|---|
| `--state-error` | `#E5533D` | 4.67:1 | Error states only |
| `--state-ok` | `#5FA86B` | 6.05:1 | Confirmed or running only |

These do not count against the palette limit (R-29). Reason: they are
functional signals required by the error and empty states (R-27), never
decoration, and never used to color anything that is not a state.

### Palette discipline

Core palette is the neutral ramp plus exactly one accent. Copper appears on
live values, the active item, and the primary action. It does not appear on
every icon, every border, and every heading at once. If copper is everywhere,
it has stopped being an accent.

## 4. Typography **[owner]** + **[derived]**

**[owner]** Barlow and Barlow Condensed.

Chosen for character, not as a default pick: Barlow is a grotesque with
slightly squared corners drawn in the spirit of transit and signage lettering,
which suits equipment. The condensed cut packs dense overlay labels without
shrinking the type.

**[derived]** Assignment:

- **Barlow Condensed** for section labels, panel headers, and large numeric
  readouts. Reason: it holds dense labels at a readable size in narrow columns.
- **Barlow** for body copy, controls, and anything read as a sentence. Reason:
  the condensed cut is for labels, not for prose.
- **Tabular figures** for any number that updates live. Reason: proportional
  digits make a changing value jitter horizontally.

No monospace font for headings, and no uppercase with wide tracking as a
default label style.

## 5. Mood **[derived from 1 and 2]**

Equipment, not software. The reference is a matte black capture card or a
machined peripheral: it feels solid, the edges are precise, and the finish is
the point. Restraint is what signals quality.

Not: a dark UI with a glow on everything. That is the failure mode this
direction is one step away from, and the difference is dose.

## 6. Dials **[owner]**

`Dial: ENERGY 2 / RHYTHM 2 / MOTION 1`

- **ENERGY 2.** Composed. Considered contrast, not shouting.
- **RHYTHM 2.** Consistent with a few deliberate breaks. Sections are not all
  the same composition, and they are not all different either.
- **MOTION 1.** Hover and focus states only. No scroll reveals, no parallax,
  no floating. Reason: this is a tool people leave open for hours, and motion
  in a persistent tool becomes irritation rather than delight.

MOTION 1 is a hard constraint on the build, not a preference (R-19).

## 7. Dark theme

Dark is the default and, for now, the only theme. This is a stated reason, not
an aesthetic default (R-21): the product renders on top of live gameplay, so it
cannot throw light at the player, and the shell has to match the overlay it
controls. If a light theme is ever added, both modes have to be fully correct
(R-34).

## 8. Identity motif **[derived]**

**The machined edge.** A single hairline (`--border-hairline`) at the top edge
of raised surfaces, reading as the lit edge of a bevel. It is used consistently
and never doubled into a full outline plus glow.

One motif, repeated. That repetition is what makes the design belong to this
product rather than to any dark app.

## 9. The mark **[owner chose the name, agent drew the mark on request]**

Name: **Stasis**, the owner's choice.

**[derived]** The reading the mark and the copy work from: stasis is a state
held without change, and that is what the product promises the game underneath.
Against an incumbent known for its weight, the name commits to the thing being
measured in the footprint bar.

The mark is four viewfinder corner brackets framing a single copper square.

- **Not an eye.** An eye mark on a product whose pitch is "this is not spyware"
  argues against itself, and eye logos are the most worn shape in the category.
- **Brackets, not a crosshair.** There is no cross, so it reads as framing and
  holding rather than aiming, and it does not narrow the product to a shooter
  overlay. Brackets that hold something in place without touching it are the
  closest shape to what the name claims.
- **The centre square is the status square.** The same glyph marks state in the
  footer, so the icon and the interface share one motif instead of two.
- **Drawn once, used twice.** `tools/make-icon.py` renders the platform icons
  from a 1024 grid; `src/components/Mark.tsx` draws the identical geometry as
  inline SVG for the titlebar. The constants match by hand, so a change to one
  means a change to the other.
- **Weights were compared, not guessed.** Rendered side by side at 256, 32 and
  16px. Heavier strokes went chunky at large sizes; lighter ones thinned out in
  the taskbar.
- **No lit edge in the icon.** The machined bevel highlight from section 8 was
  tried and cut: at icon scale it stopped reading as a bevel and became a white
  stripe across the top.

## 10. Open questions

- Trademark and domain availability for "Stasis" have not been checked. It is
  a common English word, so it is the hardest class of name to secure, and it
  is already the title of a released adventure game.
- The overlay engine does not exist yet. This direction covers the shell only.
