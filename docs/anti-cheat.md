# Anti-cheat position

This is the document to point at if you are an anti-cheat vendor, a publisher,
or a player who wants to know what Stasis does inside a game process. It
describes the engine as decided, not as built: at the time of writing there is
no overlay engine in this repository, and `ROADMAP.md` says the same. Where the
tense below is present, read it as the contract the engine is being built to,
and hold the code to it when the code exists.

The standard this document holds itself to is the one `README.md` already
applies to telemetry: no claim the project cannot support. That rules out the
sentence most overlay products lead with. Stasis will not tell you it cannot get
you banned.

## The architecture, stated plainly

Stasis injects a DLL into the game process and hooks the graphics API's
presentation call. On Direct3D 11 that is `IDXGISwapChain::Present`. The overlay
is drawn into the game's own back buffer, in the game's own process, immediately
before the frame is presented.

This was chosen over a non-injecting transparent window. The reasoning and the
trade are recorded in `ROADMAP.md` and `TODO.md` section 4. It is the same
technique the incumbent uses: Overwolf's own overlay API reference describes
injecting overlays into supported games, exposes `requestGameInjection()`, and
lists D3D9, D3D11, D3D12 and Vulkan backends
(https://dev.overwolf.com/ow-electron/reference/Overwolf-electron-APIs/overlay/interfaces/IOverwolfOverlayApi/).

## What Stasis does not do

Each item is a line the code is held to, not a description of what happens to be
absent because nothing is written yet.

- **Does not read game memory.** No scanning, no pointer chasing, no reading of
  game state for display or for anything else. The only bytes Stasis touches are
  the ones it writes into the back buffer.
- **Does not write game memory,** other than the hook itself: the function
  pointer or prologue that redirects presentation, and the graphics state saved
  and restored around drawing.
- **Does not send synthetic input.** No `SendInput`, no posted or injected window
  messages to the game, no simulated clicks or key presses. Hotkeys are
  registered with the OS and consumed by Stasis, and the input release path hands
  control back to the game rather than driving it.
- **Does not modify game files.** Nothing on disk belonging to a game or a
  launcher is written to. `CONTRIBUTING.md` already states this rule for the
  library scanner and it applies to the engine unchanged.
- **Does not load a kernel driver.** Stasis ships no `.sys` file and installs no
  service. Riot blocked MSI Afterburner's `RTCore64.sys` when Vanguard shipped,
  because a signed driver with a known privilege-escalation bug is a cheat loader
  (community reports at
  https://forums.guru3d.com/threads/rtcore64-sys-and-valorant-vanguard.431963/,
  summarised at https://en.wikipedia.org/wiki/Riot_Vanguard). A project that
  ships no driver cannot be that problem.
- **Does not require elevation.** Stasis runs as the user. If a game runs
  elevated, Stasis cannot attach to it, and the correct behaviour is to say so
  rather than to ask for administrator rights. An overlay that requests admin in
  order to get into a game process is asking for the wrong thing.
- **Does not attempt to evade or disable anti-cheat.** No unhooking, no signature
  obfuscation, no timing tricks, no packing, no attempt to hide the module from
  an enumeration of loaded modules. Stasis is meant to be findable.
- **Does not open a network socket.** No telemetry, no analytics, no phone home.
  That is a `README.md` commitment for every version, and the engine does not get
  an exemption.

## The uncomfortable part

Injected code sits inside the process address space. Nothing technical stops a
DLL that can draw into the back buffer from also reading the game's memory. The
list above is a set of promises about what the code does, not a description of
what the architecture makes impossible.

That distinction is the reason this repository is public. The promises are
checkable against the source, and `CONTRIBUTING.md` says a pull request adding
any of them is refused. If you are evaluating Stasis and this bothers you, it
should. It is the cost of the path the project chose, and writing around it would
be exactly the kind of claim this document exists to avoid.

## Residual risk, stated without hedging

Seven risks the architecture does not remove. Anyone deciding whether to run
Stasis is entitled to all of them.

**1. A publisher can block injection at any time, and one already has.** Valve
announced on 26 June 2020 that DLLs interacting with CS:GO would need an
Authenticode signature, and that Valve would block signed DLLs whose
functionality interfered with the game in any way
(https://blog.counter-strike.net/index.php/2020/06/30683/). Trusted Mode shipped
the following month as the default
(https://blog.counter-strike.net/ko/2020/07/30756/). Nobody was banned; injecting
overlays simply stopped working. `docs/when-injection-breaks.md` is the plan for
that day.

**2. A kick is not a ban, and users do not care about the difference.**
BattlEye's FAQ says they generally ban only for actual cheats, and that non-cheat
overlays are generally supported unless the game developer wants otherwise. The
same FAQ says they might kick rather than ban for a specific program, without
flagging the user as a cheater (https://www.battleye.com/support/faq/). A
repeated mid-match kick is close enough to unplayable that the distinction stops
mattering to the person it happens to.

**3. Epic's own documentation names this technique as a conflict source.** Epic
writes that software relying on undocumented operating system behaviour, using
self-modifying code, or patching system libraries is very likely to conflict with
the anti-cheat service
(https://dev.epicgames.com/docs/epic-online-services/trust-and-safety/anti-cheat-interfaces/using-anti-cheat).
Hooking `Present` is patching a system library. Epic is describing the
architecture without malice, and it is worth reading as written.

**4. Microsoft says the same about the technique.** The DirectX team writes that
intercepting the render and presentation process can cause problems including
performance regressions, instability and issues with anti-cheat
(https://devblogs.microsoft.com/directx/demystifying-full-screen-optimizations/).
That is the platform owner, not a competitor.

**5. What a module draws matters more than how it is drawn.** The best documented
case of an overlay's users being permanently banned is Overplus in Dota 2 in
early 2024, distributed through Overwolf. The banned behaviour was seeing
opponents' draft statistics, not the act of drawing on screen
(https://dotesports.com/dota-2/news/dota-2s-new-ban-wave-has-overplus-users-panicking-while-other-players-celebrate,
https://www.forbes.com/sites/mikestubbs/2024/01/14/dota-2-fans-call-for-valve-to-ban-controversial-overplus-software/).
Overwolf had the publisher relationships and its users were banned anyway. A
module that shows a player information the game did not give them is the risk,
and the module permission model in `TODO.md` section 5 is where that gets
handled.

**6. Ban waves get blamed on the most visible overlay running.** After Destiny
2's PC launch in October 2017, players attributed a ban wave to OBS, XSplit,
Fraps and similar tools. Bungie said it had not banned for those tools and
reversed a set of erroneous bans it described as unrelated to third-party
software
(https://www.pcgamer.com/bungie-third-party-apps-will-not-result-in-destiny-2-bans/,
https://www.techspot.com/news/71597-bungie-denies-claims-legitimate-third-party-apps-causing.html).
The accusation did not need to be true to be made. Stasis will be the overlay in
that thread for its users.

**7. No anti-cheat vendor has told us this is acceptable.** The research behind
this document could not find any published Easy Anti-Cheat or BattlEye programme,
process, or allowlist addressed to third-party overlay developers, and could not
find a precedent of a small open-source overlay obtaining a compatibility
arrangement. That is recorded as unverified rather than settled.
`docs/vendor-letters.md` is the attempt to settle it. Until an answer arrives,
silence is silence, and it is not permission.

## What this document will not say

Overwolf's support site states that Overwolf and all official Overwolf apps will
not get you banned
(https://support.overwolf.com/support/solutions/articles/9000182312-overwolf-won-t-get-you-banned).
Read closely, the guarantees on that page and on
https://www.overwolf.com/our-commitment/ are about publisher terms of service and
app store curation, and neither page names an anti-cheat vendor. Stasis has
neither the curation pipeline nor the publisher agreements, so the equivalent
sentence here would be worth less than nothing.

Stasis will not claim to be ban-safe. It publishes what it does in enough detail
to be checked, and leaves the decision with you.

## Titles Stasis will not support

Riot's third-party developer FAQ states that there is absolutely no allow list
for Vanguard, that nobody is exempt, and that external tools reading memory will
no longer work. The same document says overlays and internal tools using the API,
game client and in-game APIs should continue to function
(https://www.riotgames.com/en/DevRel/vanguard-faq). Stasis is neither of those.
It is injected code, which is the side of Riot's line that does not work.

Valorant, League of Legends and Teamfight Tactics are therefore listed as will
not be supported in `docs/supported-games.md`. That is a decision, not a gap
waiting to be filled.

## If you believe Stasis got you banned

The order matters, so read this before you appeal.

**Appeal to the publisher first, and say what was running.** They are the only
party who can reverse a ban. Nobody working on Stasis can see your account, act
on your behalf, or influence the outcome. Tell them Stasis was running. Hiding it
makes an appeal worse, and this project will never ask you to.

**Then tell us, with enough detail to be useful.** Open an issue on
https://github.com/Jrnilsen01/Stasis with:

- the game, the platform, and the date and time as closely as you can place it
- the exact wording of the ban or kick message
- whether Stasis was attached at the time, and to which process
- which modules were enabled, and what each one displayed
- `logs/stasis.log` from the app data folder. It is written locally, capped at a
  megabyte, and paths under your user folder are already written as `~`, so it is
  meant to be attachable to a public issue. Read it before you post it anyway.
- the publisher's reply, if you get one

Redact your account name if you want to. It is not needed to investigate.

**What happens next.** If a pattern appears for a title, that title moves to
known broken or to will not be supported in `docs/supported-games.md`, with the
reason written next to it. Cases where Stasis was probably at fault are published
in the same place as the ones where it was not. A supported-games list that only
ever gains entries is not a list anybody should trust.

**What will not happen.** No response-time guarantee, no compensation, no appeal
filed on your behalf, and no claim from this project that the ban was definitely
not Stasis. One person maintains this in the open, and `SECURITY.md` already sets
the same expectation for vulnerability reports.

## Sources

Vendor and platform owner:

- Riot Vanguard third-party FAQ: https://www.riotgames.com/en/DevRel/vanguard-faq
- BattlEye FAQ: https://www.battleye.com/support/faq/
- Epic Online Services, using the anti-cheat interfaces: https://dev.epicgames.com/docs/epic-online-services/trust-and-safety/anti-cheat-interfaces/using-anti-cheat
- Microsoft DirectX team, Demystifying Fullscreen Optimizations: https://devblogs.microsoft.com/directx/demystifying-full-screen-optimizations/
- Valve, Interacting with CS:GO, 26 June 2020: https://blog.counter-strike.net/index.php/2020/06/30683/
- Valve, Trusted Mode, July 2020: https://blog.counter-strike.net/ko/2020/07/30756/
- Overwolf, our commitment: https://www.overwolf.com/our-commitment/
- Overwolf, Overwolf won't get you banned: https://support.overwolf.com/support/solutions/articles/9000182312-overwolf-won-t-get-you-banned
- Overwolf, overlay API reference: https://dev.overwolf.com/ow-electron/reference/Overwolf-electron-APIs/overlay/interfaces/IOverwolfOverlayApi/

Reporting by named outlets:

- Dot Esports, Overplus ban wave: https://dotesports.com/dota-2/news/dota-2s-new-ban-wave-has-overplus-users-panicking-while-other-players-celebrate
- Forbes, Dota 2 fans call for Valve to ban Overplus: https://www.forbes.com/sites/mikestubbs/2024/01/14/dota-2-fans-call-for-valve-to-ban-controversial-overplus-software/
- PC Gamer, Bungie says third-party apps will not result in Destiny 2 bans: https://www.pcgamer.com/bungie-third-party-apps-will-not-result-in-destiny-2-bans/
- TechSpot, Bungie denies claims: https://www.techspot.com/news/71597-bungie-denies-claims-legitimate-third-party-apps-causing.html

Community reports, cited as reports rather than as policy:

- guru3D, RTCore64.sys and Vanguard: https://forums.guru3d.com/threads/rtcore64-sys-and-valorant-vanguard.431963/
- Wikipedia, Riot Vanguard: https://en.wikipedia.org/wiki/Riot_Vanguard
