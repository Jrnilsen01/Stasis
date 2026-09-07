# HDR and capture

Two behaviours that follow from how the overlay is drawn, and that get decided by
accident if they are not decided on purpose. `TODO.md` section 4 lists both. Both
are cheap to settle now and expensive to change after the hook matures and people
have built expectations on top of it.

---

# Part 1: HDR

## The mechanism

Stasis draws into the game's back buffer. That buffer has a pixel format and a
colour space, both chosen by the game, and the values Stasis writes are
interpreted according to them. An overlay authored in ordinary sRGB and written
straight into an HDR swapchain will be wrong, and it will be wrong in a specific,
predictable way rather than randomly.

Three swapchain configurations matter.

**SDR.** `DXGI_FORMAT_R8G8B8A8_UNORM`, `DXGI_FORMAT_B8G8R8A8_UNORM`, or their
`_SRGB` variants. Eight bits per channel, sRGB transfer, Rec.709 primaries. This
is the case an overlay is implicitly written for, and the only thing to get right
is whether the render target view is the sRGB variant, which decides whether the
hardware applies the transfer function on write.

**HDR10.** `DXGI_FORMAT_R10G10B10A2_UNORM` with
`DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020`. Ten bits per channel, PQ transfer
function, Rec.2020 primaries. PQ encodes absolute luminance rather than a
relative signal, and the transfer function and primaries are defined in ITU-R
BT.2100 (https://www.itu.int/rec/R-REC-BT.2100).

**scRGB.** `DXGI_FORMAT_R16G16B16A16_FLOAT` with
`DXGI_COLOR_SPACE_RGB_FULL_G10_NONE_P709`. Half-float, linear transfer,
Rec.709 primaries, where 1.0 is the sRGB reference white of 80 cd/m². Values above
1.0 are legal and are how highlights get brighter than paper white.

The colour space enumeration is documented at
https://learn.microsoft.com/en-us/windows/win32/api/dxgicommon/ne-dxgicommon-dxgi_color_space_type
and the format enumeration at
https://learn.microsoft.com/en-us/windows/win32/api/dxgiformat/ne-dxgiformat-dxgi_format.

### Why "washed out" is the usual symptom

Take an sRGB overlay, gamma-encoded, and write its values unchanged into a linear
scRGB buffer. The stored value is now treated as linear light. For any input
between 0 and 1, the gamma-encoded value is larger than the linear one it
represents, so every midtone is displayed brighter than intended. Blacks lift,
contrast collapses, and the overlay looks washed out. That is not a vague
complaint about HDR; it is what happens when a transfer function is skipped, and
it is reproducible.

The same overlay written into an HDR10 PQ buffer is wrong twice over. The values
are decoded as PQ, which maps them to absolute luminance on a curve they were
never encoded for, and Rec.709 primaries interpreted as Rec.2020 pull saturated
colours outward. The result is wrong in brightness and oversaturated at once.

### The other failure, and why this architecture avoids it

The second common way overlays break HDR is by changing the swapchain: recreating
it, calling `SetColorSpace1`, or forcing a format the game did not ask for.
Drawing inside the game's existing swapchain means Stasis has no reason to do any
of that, and the rule below makes it explicit.

## What Stasis should do

**1. Detect, do not assume.**

Read the swapchain format from `IDXGISwapChain1::GetDesc1`. Read the display's
colour space from `IDXGIOutput6::GetDesc1`
(https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_6/ns-dxgi1_6-dxgi_output_desc1).

For the colour space the game set on its swapchain, no getter was found among the
DXGI interfaces reviewed while writing this, and that should be treated as
unverified rather than settled. The reliable way to know is to hook
`IDXGISwapChain3::SetColorSpace1`
(https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_4/nf-dxgi1_4-idxgiswapchain3-setcolorspace1)
alongside `Present` and record the argument. Stasis is already hooking that
interface, so this costs one more hook and removes a guess.

**2. Never touch the game's swapchain.** Do not recreate it, do not resize it, do
not call `SetColorSpace1`, do not call `SetHDRMetaData`, do not change the format.
Save and restore whatever pipeline state the draw disturbs. If the swapchain
description differs before and after Stasis attaches, that is a bug of the most
serious class, because it changes what the user's display is doing.

**3. Author once in sRGB, convert at composite time.** The host renders the
overlay UI into an offscreen sRGB texture, which is what it would produce anyway.
The injected component converts that texture to the target's space as the last
step:

- *SDR target*: write directly, respecting whether the render target view is the
  `_SRGB` variant.
- *scRGB target*: decode sRGB to linear, then scale by
  `referenceWhiteNits / 80.0`, because 1.0 in scRGB is 80 cd/m². Blend in linear,
  which this format gives for free.
- *HDR10 target*: decode sRGB to linear Rec.709, scale to nits by
  `referenceWhiteNits`, convert Rec.709 primaries to Rec.2020, then apply the
  inverse PQ transfer function.

**4. Be honest about blending on HDR10.** Correct alpha blending needs linear
light, and an HDR10 back buffer holds PQ-encoded values. Blending directly into it
is an approximation. It is fine for opaque panels and visibly wrong on soft alpha
edges and drop shadows. Two acceptable answers: design the overlay's HDR
appearance around mostly-opaque panels with hard edges, or composite into a
linear intermediate first and encode once. Pick one deliberately. The
`ENERGY 2 / RHYTHM 2 / MOTION 1` restraint in `DESIGN.md` makes the first answer
cheaper than it would be for a flashier interface.

**5. Pick a reference white, and let the user move it.** The overlay's white has
to sit at a luminance that matches the rest of the user's desktop and the game's
own menus, and no single number is right for every display.

The system's SDR content white level is the value to prefer, but the API for
reading it was not verified while writing this and should be confirmed against
the current Windows SDK before it is relied on. Note that `DXGI_OUTPUT_DESC1`
gives maximum and minimum luminance and a white point chromaticity, and does not
give the SDR white level.

Where it cannot be read, default to 80 cd/m², the scRGB reference, and expose a
slider in Settings labelled in nits. That default will look dim on many HDR
displays. Dim and adjustable is the right failure: an overlay that is too bright
on an HDR panel is unpleasant in a way that dimness is not, and the user can fix
dim in one control.

**6. Refuse to draw on a combination that is not handled.** Log the format and the
colour space and show the user the honest-degradation message rather than drawing
wrong colours over their game. `TODO.md` already names a silently absent overlay
as the worst outcome; a visibly wrong one is second worst, and both are worse than
a sentence explaining what happened.

## How to test it

**The check that needs no HDR display, and is the strongest one.** Log the
swapchain format and colour space on every attach, in every title, always. Then
assert that the description is byte-for-byte identical before and after Stasis
attaches. This catches rule 2 violations automatically, runs on any machine, and
builds a dataset of which target titles use which format as a side effect. Do this
first.

**The checks that need an HDR display.** Enable HDR in Windows display settings,
then work through four configurations:

1. SDR display, game in SDR. The baseline.
2. HDR display, game running in SDR. Windows composites SDR content into an HDR
   desktop, and the overlay should look identical to configuration 1.
3. HDR display, game in HDR10.
4. HDR display, game in scRGB.

For 3 and 4, three things to look at:

- **Does the game's HDR still work?** Bright highlights should still be brighter
  than white. If enabling the overlay flattens them, something changed the
  swapchain.
- **Does the overlay's white match the game's own menu white?** This is the
  reference-white calibration and it is a judgement made with eyes, not a
  measurement.
- **Do the overlay's edges and shadows look right?** This is the blending
  question from rule 4, and HDR10 is where it shows.

**A warning about screenshots.** A Windows screenshot of HDR content is tone
mapped on the way out, so comparing HDR output through SDR screenshots will
mislead you. Judge configurations 3 and 4 on the display itself. Use screenshots
only for the SDR configurations, where they are reliable.

The Windows HDR Calibration app from the Microsoft Store is worth running once to
get the display itself into a known state before any of this. It calibrates the
display, not the overlay, and it is a prerequisite rather than a test.

---

# Part 2: Capture

## What the architecture already decides

Stasis draws into the game's back buffer before `Present` returns. Everything
downstream of that point sees one composited frame, and cannot separate the
overlay from the game, because by then there is nothing to separate.

That means the overlay appears in:

- **OBS Display Capture and Window Capture**, which take what the desktop
  compositor shows.
- **NVIDIA ShadowPlay** and **Xbox Game Bar** recordings, which capture the
  presented frame.
- **Anything else that records the presented frame,** including whatever an
  anti-cheat screenshot module captures.

Two cases are less simple and are worth knowing about.

**OBS Game Capture** hooks the game itself and copies the back buffer. Two
independent hooks on the same presentation call, in the same process, is a real
compatibility surface: whether the overlay lands in the copy depends on hook
order, and the two hooks coexisting at all is not guaranteed. This belongs in
`docs/supported-games.md` as a per-title note once it has been tested, and it is
worth testing early because OBS Game Capture is common.

**In-engine screenshots** taken by the game itself may or may not include the
overlay, depending on whether the game grabs the back buffer before or after
`Present`. This is per-title and not worth engineering around.

## The default: the overlay appears in captures

Four reasons, in order of weight.

**1. Making it capture-invisible is a cheat feature, in shape and in fact.** An
overlay that a human reviewer cannot see in a screenshot, while the player can see
it on their screen, is precisely how ESP evades screenshot review. Anti-cheat
practitioners describe screenshot capture as a long-standing technique for finding
exactly that, and Vanguard is community-documented as having a server-triggered
screenshot module, though no vendor documentation of the review process was found
and that should be treated as unverified
(https://x.com/AntiCheatPD/status/1786121752034050534,
https://www.neogaf.com/threads/valorant-riot-vanguard-can-now-take-screenshots-of-your-pc-screen-without-you-knowing.1670096/).

`docs/anti-cheat.md` states that Stasis makes no attempt to evade detection.
Building a mode where the overlay is deliberately invisible to capture would
contradict that line directly, and it is the single feature most likely to be
quoted back at the project.

**2. It is what the architecture gives.** Making the overlay invisible to capture
from inside the game's own swapchain would require deliberate machinery. Not
building it is the default, and defaults that require no code are the ones that
stay correct.

**3. It matches what people expect.** A recording should show what the player saw.
A streamer whose viewers see a timer the streamer was not using, or do not see one
the streamer was, is confused in both directions.

**4. It is the position to be in when asked.** If an anti-cheat screenshot lands
on a frame with a Stasis overlay in it, what the reviewer sees is what the player
saw. That is a much better conversation than explaining why the overlay is not in
the picture.

## The user setting, eventually, and its boundary

Streamers will ask for clean footage. That request is legitimate and it deserves
an answer, but not the obvious one.

**Ship: a hotkey that hides the overlay entirely.** The overlay disappears from
the player's screen and from the recording at the same time, because it stops
being drawn. This solves the actual problem, is a few lines of state, and is
already implied by the input state machine in `TODO.md`.

**Do not ship: hide from capture only.** Visible to the player, absent from the
recording, is the evasion feature described above. It should be refused, and the
refusal should be written into `CONTRIBUTING.md`'s list alongside the existing
cheat-software entries, so that it is a stated position rather than an arbitrary
rejection when someone opens the pull request.

One narrow exception, for clarity rather than as a plan. If Stasis ever draws
something in a window of its own outside the game, Windows documents
`SetWindowDisplayAffinity` with `WDA_EXCLUDEFROMCAPTURE`
(https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity)
as the supported way to keep a window out of captures. That is a reasonable thing
for a credentials dialog and is not a reasonable thing for the in-game overlay.

## What to decide before the hook matures

Five decisions, all of which get harder once there is code and users.

1. Whether the overlay renders once in sRGB and converts at composite, or renders
   per target space. The first is recommended above.
2. The HDR10 blending answer: opaque panels, or a linear intermediate.
3. The reference white default and whether it is user-adjustable. Recommended:
   80 cd/m² fallback, adjustable, prefer the system value once the API is
   confirmed.
4. That the swapchain description is asserted unchanged across attach, and that
   this assertion is part of the engine's test harness rather than a manual check.
5. That the capture default is visible, that the hotkey hides the overlay
   entirely, and that capture-invisible drawing is refused in `CONTRIBUTING.md`.

## Sources

- ITU-R BT.2100, HDR transfer functions and Rec.2020 primaries: https://www.itu.int/rec/R-REC-BT.2100
- Microsoft, DXGI_COLOR_SPACE_TYPE: https://learn.microsoft.com/en-us/windows/win32/api/dxgicommon/ne-dxgicommon-dxgi_color_space_type
- Microsoft, DXGI_FORMAT: https://learn.microsoft.com/en-us/windows/win32/api/dxgiformat/ne-dxgiformat-dxgi_format
- Microsoft, IDXGISwapChain3::SetColorSpace1: https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_4/nf-dxgi1_4-idxgiswapchain3-setcolorspace1
- Microsoft, DXGI_OUTPUT_DESC1: https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_6/ns-dxgi1_6-dxgi_output_desc1
- Microsoft, SetWindowDisplayAffinity: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity
- Microsoft DirectX graphics samples, including the D3D12 HDR sample: https://github.com/microsoft/DirectX-Graphics-Samples

Community reports, cited as reports rather than as policy:

- Anti-Cheat Police Department on screenshot capture for ESP detection: https://x.com/AntiCheatPD/status/1786121752034050534
- NeoGAF, Vanguard screenshot module discussion: https://www.neogaf.com/threads/valorant-riot-vanguard-can-now-take-screenshots-of-your-pc-screen-without-you-knowing.1670096/

The Microsoft API reference links above were written from the documented
interface names and were not fetched while writing this file. Check them before
this document is published anywhere it is quoted.
