# Roadmap

What exists, what is decided but unbuilt, and what is still an open question.

The distinction that matters most: **Stasis is an app shell today, and the
overlay engine has not been started.** Everything in the first section below
works and is tested. Everything after it is intent, and some of it is not even
that yet.

`TODO.md` holds the same picture in more detail, with the reasoning attached to
each item. `docs/` holds the positions that follow from the engine decision: the
anti-cheat stance, the supported-games policy, what happens when injection is
blocked, and the fork hazard.

## Now: the shell

The window you open outside the game. This is the whole of what the repository
contains.

- Reads the Steam library from Steam's own manifests: the install location from
  the registry, library folders from `libraryfolders.vdf`, and each title from
  `appmanifest_*.acf`.
- Lists overlay modules from folders containing a `module.json`. Nothing ships
  preinstalled, so on a fresh install this list is honestly empty.
- Two settings, both consumed by something real.
- A footprint bar measuring this process's own memory, CPU, and uptime.

Verified rather than assumed: 27 Rust tests, 27 frontend tests, and CI running
lint, both suites, and the build on Windows for every push.

## Next: the overlay engine

None of this exists yet, but the question that gated it has been answered.

An overlay that hooks a game's presentation looks, to an anti-cheat driver, much
like a cheat. Being open source cuts both ways here: auditable, but also
forkable into something that is not. Users getting banned would end the project,
which is why the posture was settled before the first line of injection code
rather than discovered afterwards.

**Decided: Stasis will hook the graphics API.** Injection into the game
process, drawing in the game's own presentation path. This was chosen over a
non-injecting transparent window, knowing the trade: parity with the incumbent
and correct behaviour in every presentation mode, paid for with a per-API
renderer to maintain, faults that land inside someone else's process, and a
narrower set of titles.

Three consequences that follow, stated here rather than discovered later:

- **Riot titles are out of scope.** Riot states there is no allow list for
  Vanguard and draws its line at injected code. Valorant and League will not be
  supported.
- **Injection will break on somebody's patch day.** Valve's CS:GO Trusted Mode
  required signed DLLs and blocked anything interfering with the game, and
  overlays stopped working overnight. Stasis needs to detect that state and say
  so, rather than appearing broken.
- **There will be a supported-games list, and it will have gaps.** Per-title
  quirks are unavoidable on this path. The list is the honest form of that.

Still open, in the order they block each other: which graphics API ships first
(one that works beats five that half work), the injection mechanism and its
failure path, the process model, hotkeys with a guaranteed input release, and
crash isolation so a fault disables the overlay rather than killing the game
twice.

## After: the module system

A module is meant to be one overlay panel: a crosshair, a timer, a capture
trigger. The folder format already exists in a minimal form, and the moment
someone writes against it, it becomes a contract.

It is blocked on one question: what a module actually is. Static HTML and JS in
a webview, a WASM module, or a native plugin. The sandbox and the permission
model follow from that answer rather than the other way round, and they are not
optional. A module that can read the screen and the filesystem is a keylogger
waiting to happen, and "no telemetry, no ads, no account" dies the first time a
module exfiltrates something.

## In parallel: coverage and trust

- **More launchers.** The scanner reads Steam only. Epic, GOG, Xbox and Game
  Pass, Battle.net, EA, and Ubisoft are all unhandled. This one genuinely does
  not wait for the engine.
- **Code signing.** This stopped being a distribution nicety when the injection
  path was chosen. Some anti-cheat configurations refuse an unsigned injected
  DLL outright, so signing now gates the engine rather than the installer.
  SmartScreen warning on an unsigned installer is the smaller half of the
  problem.
- **Auto-update**, which needs the signing key handling settled first.
- **Logging and settings validation**, both done. An accessibility pass, text
  scaling, and the overlay's own contrast problem remain.

## Not on this roadmap

**Dates.** One person, before any release, with the largest question still
unanswered. A date would be a guess wearing the clothes of a plan, and the
footprint bar exists precisely because this project prefers a measured number to
a confident claim.

**Anything that makes this work as cheat software.** See `CONTRIBUTING.md`.

**Platforms other than Windows.** `steam.rs` contains macOS and Linux path
detection that has never been run. Either it gets tested or it gets removed;
until then it is not a supported platform and will not be described as one.
