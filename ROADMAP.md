# Roadmap

What exists, what is decided but unbuilt, and what is still an open question.

The distinction that matters most: **Stasis is an app shell today, and the
overlay engine has not been started.** Everything in the first section below
works and is tested. Everything after it is intent, and some of it is not even
that yet.

`TODO.md` holds the same picture in more detail, with the reasoning attached to
each item.

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

None of this exists. It is gated on a question that is not primarily technical.

An unknown overlay that hooks a game's presentation looks, to an anti-cheat
driver, much like a cheat. Being open source cuts both ways here: auditable, but
also trivially forkable into something that is not. Users getting banned would
end the project, so the posture has to be decided before the first line of
injection code, not discovered afterwards.

In the order they block each other:

1. **Anti-cheat strategy.** Pursue vendor relationships, restrict to titles
   without kernel anti-cheat, or use only officially sanctioned overlay APIs.
2. **Rendering path.** Hooking D3D11/D3D12/Vulkan/OpenGL, against a transparent
   click-through always-on-top window. The second is far safer and much worse
   for latency and exclusive fullscreen.
3. **Process model.** An in-process hook, or a separate process compositing over
   the game. Separate is safer and slower.
4. **Hotkeys**, including a toggle that is guaranteed to hand input back.
5. **Exclusive fullscreen against borderless**, per game.

Deciding 2 through 5 before 1 would be guessing at constraints that answer 1
sets.

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

None of these wait for the engine.

- **More launchers.** The scanner reads Steam only. Epic, GOG, Xbox and Game
  Pass, Battle.net, EA, and Ubisoft are all unhandled.
- **Code signing.** Installers are unsigned, so SmartScreen warns every user.
  For a product whose argument is trustworthiness, that matters more than usual.
- **Auto-update**, which needs the signing key handling settled first.
- **Logging, settings validation, and an accessibility pass.**

## Not on this roadmap

**Dates.** One person, before any release, with the largest question still
unanswered. A date would be a guess wearing the clothes of a plan, and the
footprint bar exists precisely because this project prefers a measured number to
a confident claim.

**Anything that makes this work as cheat software.** See `CONTRIBUTING.md`.

**Platforms other than Windows.** `steam.rs` contains macOS and Linux path
detection that has never been run. Either it gets tested or it gets removed;
until then it is not a supported platform and will not be described as one.
