# docs

Policy and planning for the overlay engine. The engine does not exist yet, so
everything here is written before the code rather than after it. `ROADMAP.md`
holds the shape of the project; `TODO.md` section 4 holds the item list these
documents were written against.

Two of these are public-facing and two are for the owner. The rest are working
documents that happen to be readable.

**[anti-cheat.md](anti-cheat.md)** is the project's public position. What Stasis
does inside a game process, what it does not do, the residual risk stated without
hedging, and what to do if you believe Stasis got you banned. This is the document
to point a vendor, a publisher, or a worried user at.

**[supported-games.md](supported-games.md)** is the supported-games policy and the
list. Four tiers, the criteria for each, and how a title moves between them. The
list is empty because there is no engine; Riot titles are listed as will not be
supported, with the reason.

**[when-injection-breaks.md](when-injection-breaks.md)** is the patch-day plan.
How Stasis detects it has been blocked instead of appearing broken, the exact
words the user sees, the rules that keep the game running whatever happens to the
overlay, and how a fix reaches people with no telemetry channel to push to.

**[fork-hazard.md](fork-hazard.md)** is an options paper for the owner. An
Apache-2.0 repository with working `Present` hooks is a cheat scaffold. Four
options, their real costs, and a recommendation to accept or reject. Not a
decision.

**[measuring-presentation-modes.md](measuring-presentation-modes.md)** is a
method the owner can run in an afternoon, using PresentMon, to find out how many
target titles actually present via exclusive fullscreen. Nobody has run it and the
result tables are empty.

**[vendor-letters.md](vendor-letters.md)** holds two drafts, one to BattlEye and
one to Epic, describing the architecture and asking whether they see a concern.
Drafts only. Nothing has been sent, the sender details are blanks, and only the
owner should send them.

**[hdr-and-capture.md](hdr-and-capture.md)** covers two behaviours that get
decided by accident if nobody decides them on purpose: what the overlay does to
HDR output, and whether it appears in OBS, ShadowPlay and Game Bar recordings.
Both get a recommended default and a reason.
