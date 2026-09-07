# Stasis

A desktop app shell for a game overlay host, built as an alternative to
Overwolf. Named for the state it promises to keep the game in: running exactly
as it would with nothing attached.

This repository currently contains **the app shell only**: the window you open
outside the game. There is no overlay engine, no injection, and no module
runtime yet. Nothing in the UI pretends otherwise. `ROADMAP.md` separates what
works from what is merely intended.

## No telemetry, no ads, no account

There is nothing to sign up for, no analytics call on launch, and no ad slot
anywhere in the design. Settings are written to a JSON file next to the app
data and stay there.

That holds for every version, not only this one. If it ever stops being true,
this paragraph is the first thing that has to change.

## This is not a cheat

An overlay that will one day draw on top of a running game sits close enough to
that category to be worth saying plainly. Stasis will not read another process's
memory for gameplay advantage, and it will not show a player information the
game has not already given them. Contributions that add either will be refused.

`CONTRIBUTING.md` says the same thing for pull requests, and `SECURITY.md` for
reports.

## What actually works

Every screen is wired to something real. There is no sample data anywhere.

- **Footprint bar.** The memory, CPU, and uptime figures in the bottom bar are
  measured from this process on every poll with `sysinfo`. The bar exists
  because "lighter than the incumbent" is the product's whole argument, and an
  argument like that belongs on screen rather than in a marketing claim.
- **Games.** Reads Steam's own manifests off disk: the install location from
  the registry, the library folders from `libraryfolders.vdf`, and each title
  from `appmanifest_*.acf`. Names, app IDs, sizes, and last-played dates are
  Steam's own values. A game with no recorded play time says "never played".
- **Modules.** Lists folders containing a `module.json` manifest under the app
  data directory. Nothing ships preinstalled, so this list is empty on a fresh
  install and the screen says so plainly.
- **Settings.** Two preferences, both consumed by something: the Steam folder
  override feeds the scanner, and the sample rate drives the footprint poll.
  Stored as JSON next to the app data. Nothing is sent anywhere.

## What is deliberately absent

- No "enable overlay" or "launch at startup" toggle. There is no engine behind
  either one yet, and a switch that flips without doing anything is a dead
  control dressed as a feature.
- No statistics, testimonials, or performance claims about Overwolf. The
  footprint bar shows this app's own measured cost and lets the reader draw the
  comparison themselves.

## Running it

Requires Node and a Rust toolchain (both present at time of writing: Node 25.9,
Rust 1.95).

```
npm install
npm run tauri dev      # development, with hot reload
npm run tauri build    # release binary and installers
```

## Design

`DESIGN.md` holds the direction: identity, palette with measured contrast
ratios, typography, and the ENERGY / RHYTHM / MOTION dials the build is held
to. It is authored by the product owner; the agent only formats it and derives
implementation values from it.

The palette is a warm-leaning neutral ramp plus one anodized copper accent.
Copper appears in exactly three places: the active rail item, the live memory
readout, and the single primary action on a screen.

## The mark

Four viewfinder corner brackets framing a copper square. Deliberately not an
eye: an eye on a product whose pitch is "this is not spyware" argues against
itself. The centre square is the same glyph the status bar uses for state, so
the icon and the interface share one motif.

`tools/make-icon.py` renders `app-icon.png` and the platform icons under
`src-tauri/icons`; `src/components/Mark.tsx` draws the same geometry as inline
SVG for the titlebar. Regenerate with:

```
python tools/make-icon.py
npx tauri icon app-icon.png
```

## Licence

Apache-2.0, in `LICENSE`. Chosen over MIT for the explicit patent grant, which
matters for a project that may end up in formal conversations with anti-cheat
vendors and publishers.

## Known open questions

- Trademark and domain availability for "Stasis" have not been checked.
- The copyright holder in the `LICENSE` appendix is still the upstream
  placeholder and needs a real name before the first release.
- The overlay engine itself does not exist yet; this is the shell that will
  eventually manage it.
