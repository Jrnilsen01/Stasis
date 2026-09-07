# When injection breaks

Injection will stop working in some title, on a day nobody at this project
chose. This document is the plan for that day, written before the engine exists
so it can shape the engine rather than be bolted on after the first incident.

## The precedent

Valve announced on 26 June 2020 that all DLLs interacting with CS:GO would need
an Authenticode signature, and that Valve would block signed DLLs if their
functionality interfered with the game in any way
(https://blog.counter-strike.net/index.php/2020/06/30683/). Trusted Mode shipped
the following month and became the default
(https://blog.counter-strike.net/ko/2020/07/30756/).

The shape of that outcome is the thing to learn from. Nobody was banned.
Injecting overlays stopped working overnight, the fallback launch option carried
a trust-score penalty, and users blamed the overlays rather than the change. The
overlays that handled it worst were the ones that just stopped appearing, because
a silent absence is indistinguishable from a bug in the overlay.

Two more sources say the same risk exists from the other direction. Epic writes
that software patching system libraries is very likely to conflict with the
anti-cheat service
(https://dev.epicgames.com/docs/epic-online-services/trust-and-safety/anti-cheat-interfaces/using-anti-cheat).
BattlEye writes that non-cheat overlays are generally supported unless the game
developer wants otherwise, which hands the decision to the publisher
(https://www.battleye.com/support/faq/). Neither is a threat. Both are notice.

## Detecting a block instead of appearing broken

The failure that matters is the silent one. Stasis has to be able to say which
stage failed, and it has to be able to tell "blocked" apart from "broken".

### Instrument every stage

Attaching is a sequence, and each step gets an explicit outcome recorded to
`logs/stasis.log` with a stage name and the underlying error. The stages, in
order:

1. **Target identified.** The process is found and matched to a known game.
2. **Process opened.** Failure here with an access-denied error is the clearest
   single signal available. It means the operating system or a protection layer
   refused, and it is what an elevated target and a protected target both look
   like from outside.
3. **Payload written and the loader entered.** Failure here after the process
   opened successfully is unusual and points at a protection layer rather than a
   bug.
4. **DLL loaded.** A load refused after a successful write is the signature or
   binary-policy case: the Trusted Mode shape. The exact Win32 error for a load
   blocked by an image-signature policy should be captured verbatim rather than
   translated, and the mapping from error code to user-facing message confirmed
   against a real failure before it is trusted. Treat that mapping as unverified
   until then.
5. **Handshake received.** The injected DLL reports back to the host. No
   handshake within the timeout, after a successful load, means the DLL loaded
   and was then stopped or unloaded.
6. **Hook installed.** The presentation call was successfully redirected.
7. **First frame seen.** The hook was called at least once.

Stage 7 is the one that catches the quiet blocks. A hook that installs and is
never called, while the game is demonstrably rendering, is a different failure
from a hook that never installed, and the user needs a different sentence for
each.

### Correlate before concluding

A single failure is not evidence of a block. Three correlations turn it into one,
and all three use information Stasis already has locally.

**Across titles on the same machine.** If attach succeeds for other titles in the
same session and fails only for this one, the machine is fine and the title is
the variable.

**Across game builds.** The Steam scanner already reads `appmanifest_*.acf`,
which carries the build id. Recording the build id alongside each attach outcome
means Stasis can say "this worked on the previous build and stops at stage 4 on
this one", which is the single most useful sentence a user can carry into a
support thread.

**Across Stasis versions.** If the title's build did not change and the Stasis
version did, the regression is ours and the message should say so.

### Guard against the crash loop

If the game process exits within a few seconds of a successful attach, twice in a
row, Stasis stops attaching to that title and records the reason. Two crashes are
enough. A user whose game dies three times because an overlay kept trying is not
going to read the third message.

## What the user is told

The message has to name the game, name the stage in words, say whose problem it
is, and say what to do. It must not apologise in the abstract, must not imply the
user did something wrong, and must not imply the game is broken.

Words that do not appear in any of these: "Oops", "Something went wrong",
"Unexpected error", "Please try again later", "Unsupported" without a reason.

**Refused at process open or DLL load.**

> Stasis could not attach to *[game]*. The game refused it at the point where the
> overlay would load, which usually means the game or its anti-cheat blocks
> overlays that inject. Stasis will not try to work around that.
>
> *[game]* is running normally. Nothing about it has changed.
>
> See the supported games list for where this title stands.

**Attached, no frames.**

> Stasis attached to *[game]* but has not been given a frame to draw on, so the
> overlay will not appear. The game is unaffected and Stasis has stopped trying
> for this session.

**A title that used to work.**

> *[game]* updated to build *[id]*, and Stasis can no longer attach to it. Game
> updates block overlays fairly often. There is nothing wrong with your machine
> and nothing for you to fix.
>
> This title has been moved to known broken. Whether it comes back depends on why
> it broke, and that is written on the list.

**A fault inside the hook.**

> Stasis hit a fault while drawing in *[game]* and switched the overlay off for
> the rest of this session. *[game]* is still running. Restart the game to try
> again.

**Repeated crash near attach.**

> *[game]* exited seconds after Stasis attached, twice. Stasis has stopped
> attaching to this title so it does not happen a third time. You can turn it
> back on in Settings.

**A game that runs elevated.**

> *[game]* is running with administrator rights and Stasis is not, so it cannot
> attach. Stasis will not ask for administrator rights to get into a game
> process.

Every one of these appears in the app, in the place the user is already looking,
and is written to the log with the stage name and the raw error. `README.md`
already commits to the log existing to be attached to a public bug report.

## Failing safe

Six rules. The first one outranks the rest.

**1. The overlay never prevents the game from running.** Stasis attaches to a
game that is already running. It is not a launcher, it does not wrap or replace
the game's executable, it does not place a DLL next to the game's binary, and it
does not modify anything the game loads. If Stasis is uninstalled mid-session,
the game keeps running.

**2. Every stage has a timeout and gives up.** No unbounded wait, no retry loop
without a ceiling. Failure to attach ends in a recorded terminal state, not in a
thread still trying.

**3. A fault unhooks and never re-arms.** After a fault in the injected code, the
hook restores the original call path, the module goes inert, and it does not
attempt to install again in that process. `TODO.md` states this as disabling the
overlay rather than crashing the same game twice.

**4. The render thread does the minimum.** No allocation, no file access, no
blocking call on anything the host process holds, no lock that the host can also
take. A hung overlay that stalls presentation is a frozen game, and a frozen game
is worse than no overlay.

**5. The way out does not run through the thing that is broken.** A file in the
app data folder and a command line flag, both checked before any injection is
attempted, both disabling the engine entirely. Neither requires the app's UI to
be responsive. `TODO.md` asks for exactly this.

**6. Unknown means off.** If Stasis cannot determine whether it is attached, it
behaves as if it is not. Ambiguity resolves toward doing nothing.

## How a fix ships

**Classify first, in public.** Three classes, and the supported-games entry says
which one applies:

- *Our bug.* Stasis broke it. Fixable, and the fix is ours to write.
- *The game changed.* A new build moved something the hook depended on. Fixable
  if the new shape is reachable without doing anything on the not-doing list in
  `docs/anti-cheat.md`.
- *A policy block.* The publisher or anti-cheat vendor blocked injection
  deliberately. Not fixable, and it should not be treated as a puzzle.

**A policy block is never worked around.** This is the rule most likely to be
tested by a well-meaning pull request. Defeating a deliberate block is anti-cheat
evasion, `CONTRIBUTING.md` refuses it, and doing it once would cost more than
every title it could ever recover. The response to a policy block is a tier
change in `docs/supported-games.md` and a note saying who blocked it and when.

**Reproduce off the game.** `TODO.md` asks for a harness that presents frames the
way a game does. Anything reproducible there gets fixed there. Debugging by
repeatedly launching someone's competitive game is both slow and a bad idea.

**The list is updated before the fix exists.** Moving a title to known broken is
a documentation change and should land within a day of the first credible report.
The code fix arrives when it arrives. Reversing that order means users are
reading a list that says Supported while the thing does not work, which is the
one failure mode a published list is supposed to prevent.

**Signing is on the critical path.** `TODO.md` already moves Authenticode signing
earlier because an unsigned injected DLL is refused outright by some
configurations. A fix that cannot be signed and shipped quickly is not a fix that
helps on a patch day.

## Telling users, with no channel to push to

Stasis has no telemetry, no analytics, and no account, for every version, and
`README.md` says that paragraph is the first thing that has to change if it ever
stops being true. So there is no way to notify a user who is not looking. That is
self-imposed and it is not up for renegotiation here. What follows is how to work
inside it.

**The primary channel is the machine in front of the user, and it is the fastest
one available.** Everything in the detection section runs locally. Stasis knows
it failed, knows which stage, and often knows that the game's build id changed,
without asking anyone anything. A push notification would arrive later than the
message the app can already produce by itself. This is the channel to invest in.

**The repository is the second channel.** `docs/supported-games.md` is the
canonical status, a pinned issue carries the detail for a live incident, and the
release notes carry the fix. Users reach these because the in-app message names
the list and the app links to it.

**The update check is the only outbound request, and it costs something honest.**
Auto-update is already on the roadmap and gated on signing. An update check is
not telemetry when it sends nothing about the user: no identifier, no
configuration, no game list, no request body. It still reveals the user's IP
address, the approximate time, and the version being asked about, to whoever
serves the file. Say that in Settings, in those words, rather than describing the
check as invisible.

Two options, and the recommendation:

- *Manual only.* A button in Settings that fetches a small static status file
  from the repository over HTTPS, and nothing at launch. Consistent with
  `README.md`'s "no analytics call on launch". Slowest, and the most defensible.
- *Automatic, off by default,* with the exact request shown in Settings before it
  is turned on.

Ship manual only in the first version that has an engine. Revisit when
auto-update lands, and if the answer changes, change the README paragraph first
and the code second.

**The status file informs, it never controls.** A static file listing tier
changes is fine. A remotely fetched blocklist that disables titles, a
server-issued kill switch, or a forced update is a control channel, and a project
whose argument is that it does not talk to a server should not build one. The
distinction is that the user reads the file and decides. Stasis does not act on
it.

**The honest cost.** Some users will run a broken version for weeks because
nothing told them. That is the price of the commitment, the project took it
deliberately, and the way to reduce it is to make the local message good enough
that they never needed the push in the first place.

## Sources

- Valve, Interacting with CS:GO, 26 June 2020: https://blog.counter-strike.net/index.php/2020/06/30683/
- Valve, Trusted Mode, July 2020: https://blog.counter-strike.net/ko/2020/07/30756/
- Epic Online Services, using the anti-cheat interfaces: https://dev.epicgames.com/docs/epic-online-services/trust-and-safety/anti-cheat-interfaces/using-anti-cheat
- BattlEye FAQ: https://www.battleye.com/support/faq/
