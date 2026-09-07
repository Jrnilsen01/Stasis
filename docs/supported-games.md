# Supported games

## The list is empty, and that is accurate

There is no overlay engine in this repository yet. Nothing hooks a game, nothing
injects, nothing renders on top of anything, so no title has been tested and no
title is supported. `ROADMAP.md` and `CONTRIBUTING.md` say the same thing about
the engine.

The tables below are empty because there is nothing to put in them, not because
data failed to load. When the first title is verified, it appears in Supported
with a date and a version, and this paragraph gets rewritten rather than quietly
deleted.

The one section that is not empty is Will not be supported. Those entries are
decisions taken in advance, and they are listed now so nobody discovers them by
trying.

## Why publish a list at all

Universal support is not a claim this architecture can make. Hooking a game's
presentation call means per-title behaviour: different graphics APIs, different
swapchain setups, different anti-cheat configurations, different launcher
wrappers, different windowing behaviour on alt-tab. Overwolf, doing the same
thing with a decade of engineering behind it, maintains a supported-games list
rather than claiming every game works.

The alternative to a published list is not broader support. It is a user
installing Stasis, launching a game, seeing nothing, and having no way to tell a
missing feature from a bug. `TODO.md` already names that as the worst version of
the failure. A list makes the limit visible before the download rather than after
it.

`CONTRIBUTING.md` refuses "a plausible-looking wrong value where absent is the
honest answer". This document is that rule applied to a whole feature.

## The four tiers

### Supported

The overlay attaches, draws, and releases input correctly on a current build of
the game, and somebody has verified it by playing rather than by reading logs.

To enter this tier a title needs all of the following:

- attach succeeds from a cold start of the game, and after alt-tab, minimise,
  resolution change, and moving the window to another monitor
- the overlay draws correctly in the presentation modes the title offers, or the
  modes that do not work are named in the entry
- the input toggle releases control back to the game every time, including after
  the overlay itself has been forced to shut down
- the game exits normally with the overlay attached, and detach on game exit
  leaves no process behind
- no crash attributable to Stasis across the test session
- the measured frame cost is inside the performance budget, once `TODO.md`
  section 4 sets one

Each entry records the game version or build tested, the graphics API, the
anti-cheat present if any, the Stasis version, and the date.

### Known broken

The engine tries and fails, or attaches and then misbehaves. This tier exists so
that a title with a known problem is not silently missing.

An entry names the symptom, and where it is understood, the cause. Categories
seen in practice on this architecture: injection refused, hook installed but the
presentation call is never intercepted, overlay draws in the wrong colour space,
overlay draws but input never returns, game crashes on attach, anti-cheat kicks
the session.

A title sits here while the cause is being worked on. It is not a promise of a
fix. If a fix is not coming, the entry says so and the title moves to Will not be
supported.

### Will not be supported

A decision, not a backlog item. A title enters this tier when supporting it would
require something Stasis will not do, or when the publisher or anti-cheat vendor
has stated a position that Stasis's architecture does not satisfy.

Entries here are permanent until the stated reason changes. Each one carries the
reason and a source.

**Riot Games titles: Valorant, League of Legends, Teamfight Tactics.**

Riot's third-party developer FAQ states that there is absolutely no allow list
for Vanguard, that Riot cannot carve out loopholes or perform secret handshakes,
and that external tools reading memory will no longer work and developers will
need to change methods. The same document says that if a developer cannot
restructure around that, the tool is not something that will be allowed
(https://www.riotgames.com/en/DevRel/vanguard-faq).

Stasis is injected code inside the game process. Riot draws its line at injected
code and memory reading, and Stasis is on the wrong side of it. Riot also states
that they cannot support third-party developers individually, so there is no
application to file and nobody to ask.

`docs/anti-cheat.md` records the same decision. It is repeated here because this
is the document a user checks before installing.

### Untested

Everything else. This is the default state of every game in existence, and it is
where the honest answer for most titles lives.

Untested is not a synonym for unsupported. It means nobody has run it, and no
claim is being made either way. A user is free to try, and the result is worth
reporting.

## How a title moves between tiers

**Untested to Supported.** Somebody runs the checklist above on a current build
and reports the result with the version numbers. The maintainer confirms it, or
says what is still missing. A single positive report is enough to list a title as
Supported with the version tested, because the entry records what was tested
rather than a guarantee about what will work tomorrow.

**Untested to Known broken.** One reproducible failure report with logs. The
threshold is deliberately low, because a title that fails for one person will
fail for others and there is nothing to lose by saying so early.

**Supported to Known broken.** Any credible report that a previously working
title has stopped working. This is the transition that matters most, because it
is the one that happens on a patch day nobody chose. `docs/when-injection-breaks.md`
covers how Stasis detects it and what the user is told. The list is updated
before the fix ships, not after.

**Known broken to Supported.** A fix ships, and the title is retested against a
current build. The entry keeps a note of what broke and when, because a title
that has broken once is more likely to break again and users deserve to see the
history.

**Anything to Will not be supported.** Either the maintainer decides the fix
requires something the project will not do, or a publisher or anti-cheat vendor
states a position that rules the title out. The entry names which of the two it
was.

**Out of Will not be supported.** Only if the stated reason changes at the
source. A Riot policy change would qualify. A community claim that it works
anyway does not.

## Reporting a result

Open an issue on https://github.com/Jrnilsen01/Stasis. The useful reports say
which GPU and driver version, which game and launcher, the game's graphics API
and presentation mode if you know it, the Stasis version, and what the footprint
bar read at the time. `CONTRIBUTING.md` asks for the same fields for ordinary
bugs.

If the report is about a ban or a kick rather than a rendering failure, read the
last section of `docs/anti-cheat.md` first.

## Supported

None. There is no engine yet.

| Game | Graphics API | Anti-cheat | Game build tested | Stasis version | Date |
|---|---|---|---|---|---|

## Known broken

None recorded, because nothing has been tried.

| Game | Symptom | Cause, if known | First reported |
|---|---|---|---|

## Will not be supported

| Game | Reason | Source |
|---|---|---|
| Valorant | Riot states there is no allow list for Vanguard, and draws its line at injected code and memory reading. Stasis is injected code. | https://www.riotgames.com/en/DevRel/vanguard-faq |
| League of Legends | Vanguard, same statement, same reason. | https://www.riotgames.com/en/DevRel/vanguard-faq |
| Teamfight Tactics | Vanguard, same statement, same reason. | https://www.riotgames.com/en/DevRel/vanguard-faq |

## Untested

Every title not named above.
