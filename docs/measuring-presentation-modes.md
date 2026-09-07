# Measuring presentation modes

A method for the owner to run in an afternoon. It answers the open question in
`TODO.md` section 4: what fraction of the titles Stasis wants to support actually
present via exclusive fullscreen.

Nobody has run this. The tables at the end are empty and are meant to be filled
in by the person running it. The author of this document did not launch any game
and did not measure anything.

## Why bother, now that hooking is chosen

Hooking handles every presentation mode, so this measurement no longer decides
the rendering path. It still earns its afternoon, for three reasons.

It sizes the benefit. Exclusive fullscreen coverage is a large part of what the
harder path buys. If the answer turns out to be that almost nothing in the target
list uses it, that is worth knowing about a decision already taken, and worth
publishing.

It seeds the supported-games list. The presentation mode a title actually uses is
the first field of per-title behaviour Stasis will care about, and it can be
collected before any engine exists.

It records a baseline. Running the same measurement again after the engine
attaches shows whether Stasis changed the mode the game runs in, which is a
regression class that would otherwise be invisible.

## The tool

PresentMon, published by Intel at https://github.com/GameTechDev/PresentMon.

It uses Event Tracing for Windows to observe presentation, rather than injecting
into the target. Read the project's own README before running it; that file, not
this one, is the authority on what it does and what it requires. It needs an
elevated console because of the ETW session.

Two version notes that will cost time if ignored:

- **Flag syntax changed between 1.x and 2.x.** The 1.x console app takes
  single-dash long options (`-process_name`), 2.x takes double-dash
  (`--process_name`). Run `PresentMon.exe --help` once and match what it prints
  rather than copying commands from anywhere, including this document.
- **CSV column names changed too.** `PresentMode` has been stable across both.
  `Application` and `TimeInSeconds` are the 1.x spellings and may differ in 2.x.
  Open the first CSV, read the header row, and adjust the filters below to match.

Both are marked here because they are the two things most likely to make an
otherwise correct command fail.

## Choosing the target titles

Cap the list at 20. A longer list will not get finished, and 20 is enough to
produce a fraction worth quoting.

Pick by these criteria, in this order:

1. **Installed on the machine.** The Stasis Games screen already lists what Steam
   has installed, read from Steam's own manifests. Start there, because those
   titles cost nothing to launch.
2. **Actually played.** Steam publishes a most-played chart at
   https://store.steampowered.com/charts/mostplayed. A title nobody plays is not
   worth a row.
3. **Overlay-plausible.** A stats panel, a timer, or a guide has to have a use in
   the title. `ROADMAP.md` describes a module as one overlay panel. Single-player
   titles with no such use case are lower priority than long-session or
   competitive ones.
4. **Spread across graphics APIs.** Include D3D11, D3D12 and Vulkan titles, and
   one OpenGL title if the list has one. The mix matters more than the count,
   because `TODO.md` asks which API ships first.
5. **Include the suspects.** Older D3D11 titles, Vulkan and OpenGL titles, and any
   title whose display setting is called "Fullscreen" rather than "Borderless" or
   "Windowed Fullscreen". If exclusive fullscreen still exists in the target list,
   it is most likely here.

Exclude, and record the exclusion:

- Riot titles. They are out of scope by decision, per `docs/supported-games.md`.
- Anything the owner is not comfortable running third-party measurement tooling
  alongside. That is a judgement call and it belongs to the person with the
  account at risk.

Write the excluded titles and the reason into the second table. A list of 20 with
no record of what was left out is less useful than a list of 15 with one.

## What to capture per title

The question is what users actually get, so capture the default first and the
alternatives second.

- **A: default.** The title's display setting as it ships or as it was last left.
  This is the row the headline number comes from.
- **B: fullscreen.** The title set to Fullscreen or Exclusive Fullscreen, if it
  offers one. Skip if it does not, and record that it does not, because a title
  with no exclusive fullscreen option is itself an answer.
- **C: borderless.** Set to Borderless or Windowed Fullscreen.
- **D: fullscreen with Fullscreen Optimizations disabled.** Optional, and worth
  doing on two or three titles rather than all 20. Right-click the game
  executable, Properties, Compatibility, tick "Disable fullscreen optimizations".
  This is the configuration a user chasing frame times ends up in, and it is the
  most likely way a modern title still lands in true exclusive fullscreen. Untick
  it afterwards.

Sixty seconds of capture per configuration is enough. Capture during actual
rendering, not on a loading screen. Give each capture five seconds of settle time
before the numbers count, because the mode changes as the swapchain is set up.

**Do not run a capture in a ranked or competitive match.** Use a private lobby, a
training range, an offline mode, or a short session in a mode where the result
does not matter. This costs nothing and removes a whole category of regret.

## The commands

Set up once, in an elevated PowerShell, in the folder PresentMon was extracted to:

```
.\PresentMon.exe --help
mkdir C:\presentmon-results
```

Confirm the flag syntax from that help output before continuing.

Capture one title, one configuration, 2.x syntax:

```
.\PresentMon.exe --process_name game.exe --output_file C:\presentmon-results\game-A-default.csv --timed 60 --terminate_after_timed
```

The same in 1.x syntax:

```
.\PresentMon.exe -process_name game.exe -output_file C:\presentmon-results\game-A-default.csv -timed 60 -terminate_after_timed
```

If the executable name is not obvious, capture everything for 30 seconds and read
the process names out of the result:

```
.\PresentMon.exe --output_file C:\presentmon-results\survey.csv --timed 30 --terminate_after_timed
Import-Csv C:\presentmon-results\survey.csv | Group-Object Application | Select-Object Count, Name
```

Naming the files `<title>-<config>-<setting>.csv` from the start is worth the
extra keystrokes. Twenty titles times three configurations is 60 files and they
become indistinguishable within an hour.

## Reading the output

The column that answers the question is `PresentMode`. Summarise it per capture,
dropping the first five seconds:

```
Import-Csv C:\presentmon-results\game-A-default.csv |
  Where-Object { [double]$_.TimeInSeconds -gt 5 } |
  Group-Object PresentMode |
  Sort-Object Count -Descending |
  Select-Object Count, Name
```

Take the dominant mode as the title's answer for that configuration, and note the
distribution if it is not close to unanimous. Titles do switch: a menu and a match
can present differently, and alt-tab changes the mode on purpose.

### What the values mean

Confirm the exact strings against the PresentMon documentation for the version in
use; the list below is the interpretation, and the tool's own docs are the
authority on the spelling.

| PresentMode | What it means | Relevant because |
|---|---|---|
| `Hardware: Legacy Flip` | True exclusive fullscreen, flip presentation. DWM is out of the path. | This is the mode being counted. A non-injecting overlay window cannot appear over it. |
| `Hardware: Legacy Copy to front buffer` | True exclusive fullscreen, blt presentation. Also out of DWM's path. | Counts the same way. |
| `Hardware: Independent Flip` | Flip model with DWM asleep. Borderless, or fullscreen promoted by Fullscreen Optimizations. | Looks like exclusive fullscreen in frame times and is not. DWM can resume composition when something is drawn on top. |
| `Hardware Composed: Independent Flip` | As above, using a multiplane overlay plane. | Same category for this exercise. |
| `Composed: Flip` | Flip model, DWM composing. | Ordinary borderless. |
| `Composed: Copy with GPU GDI` | Blt model, DWM composing. | Older titles. |
| `Composed: Copy with CPU GDI` | Blt model through GDI, DWM composing. | Oldest path, rare. |

**The headline number.** Count the configuration-A rows whose dominant mode
starts with `Hardware: Legacy`, divide by the number of titles measured. That
fraction is the answer `TODO.md` asks for. Report it as a fraction with the
denominator visible, never as a percentage on its own, because the denominator is
20 and a percentage would imply a survey.

### What PresentMon does not tell you

It reports the presentation runtime, `DXGI` or `D3D9`, in the `Runtime` column. It
does not tell you whether a DXGI title is D3D11 or D3D12, and it does not identify
Vulkan or OpenGL by name.

To get the graphics API, look at the modules the process has loaded while it is
running: `d3d11.dll`, `d3d12.dll`, `vulkan-1.dll`, `opengl32.dll`. Task Manager's
Details tab or Process Explorer will show them. Record it as a separate column
rather than inferring it from `Runtime`.

## Recording the results

### Measured titles

| Title | Graphics API | A: default mode | B: fullscreen mode | C: borderless mode | D: FSO off | Notes |
|---|---|---|---|---|---|---|

### Excluded titles

| Title | Why excluded |
|---|---|
| Valorant | Riot title, out of scope by decision. See `docs/supported-games.md`. |
| League of Legends | Riot title, out of scope by decision. |
| Teamfight Tactics | Riot title, out of scope by decision. |

### The answer

- Titles measured: *[fill in]*
- Titles whose default mode is `Hardware: Legacy Flip` or
  `Hardware: Legacy Copy to front buffer`: *[fill in]*
- Titles offering no exclusive fullscreen option at all: *[fill in]*
- Date measured, Windows build, GPU and driver version: *[fill in]*

Record the Windows build and driver version. Fullscreen Optimizations behaviour
and multiplane overlay support both depend on them, and a result without them is
not reproducible six months later.

## Sources

- PresentMon: https://github.com/GameTechDev/PresentMon
- Steam most played chart: https://store.steampowered.com/charts/mostplayed
- Microsoft, For best performance use DXGI flip model: https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/for-best-performance--use-dxgi-flip-model
- Microsoft DirectX team, Demystifying Fullscreen Optimizations: https://devblogs.microsoft.com/directx/demystifying-full-screen-optimizations/
- Khronos, VK_EXT_full_screen_exclusive: https://registry.khronos.org/vulkan/specs/latest/man/html/VK_EXT_full_screen_exclusive.html
