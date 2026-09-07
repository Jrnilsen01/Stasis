# The fork hazard

An options paper for the owner. It ends in a recommendation, clearly marked,
which is a recommendation and not a decision.

## The problem

An Apache-2.0 repository containing working, tested `Present` hooks for one or
more graphics APIs is a good starting point for building a cheat. The hook is the
scaffold: get code running inside the game process, get called once per frame,
get a device and a back buffer, draw. Everything an ESP overlay needs after that
is game-specific work on top of a foundation the fork did not have to build.

Overwolf states this publicly as their reason for not open-sourcing: if they
open-sourced their technology, they would not be able to make sure some
developers do not use it to create cheats
(https://www.overwolf.com/our-commitment/). They are the closest thing this
project has to a peer on the technical path it chose, and this is their stated
position on exactly this question.

`TODO.md` section 4 records that the hazard went live the moment hooking was
chosen, and that the not-a-cheat statements in `README.md`, `CONTRIBUTING.md` and
`SECURITY.md` became load-bearing rather than preventative.

## Two things that are true before any option is chosen

**A licence does not stop anyone building a cheat.** It changes what they may
lawfully distribute, and nothing else. Someone who forks Stasis, adds a memory
reader, and sells the result on a subscription site is already violating game
terms of service, and in most jurisdictions is exposed to more than a copyright
claim. A copyleft licence adds one more claim to a pile they have already
demonstrated they do not care about. Nobody should choose an option here
believing it prevents cheating.

**The technique is not secret, and Stasis is not the only source.** Public
tutorials, sample code and long-running open projects already show how to hook a
presentation call and draw into a back buffer. What Stasis would add is not the
existence of the knowledge but its quality: a maintained, tested, multi-API hook
layer with the crash isolation and lifecycle handling already worked out. That is
a real uplift, and it is smaller than "we are handing out the technique". Both
halves of that sentence matter, and overstating either one leads to the wrong
choice.

## Option A: accept it and say so publicly

Keep Apache-2.0 across the whole repository. Write down, in the repository, that
the hook layer can be forked into something Stasis refuses to be, that a licence
would not prevent it, and that the project chose openness anyway.

**What it prevents.** Nothing, directly. What it does buy is the thing that
cannot be bought any other way: every claim in `README.md`, `CONTRIBUTING.md`,
`SECURITY.md` and `docs/anti-cheat.md` stays checkable against the source. "No
telemetry" and "does not read game memory" are verifiable statements rather than
assertions. For a product whose entire argument is trustworthiness, that is the
asset.

**What it does not prevent.** A closed fork sold as a cheat is entirely lawful
under Apache-2.0. There is no takedown lever, no licence violation to point at,
and no way to object beyond saying so. If a cheat named as a Stasis fork appears,
the project has a reputational problem with no legal remedy.

**Contributor goodwill.** Highest of the four. Nothing changes, nothing needs
explaining, and the licence choice already announced in `README.md` and `TODO.md`
section 1 stands. Apache-2.0's patent grant, the reason it was chosen, is kept.

**Other costs.** The reputational risk is real and unhedged. `docs/anti-cheat.md`
already tells vendors that Stasis does not read memory; the follow-up question,
"and what about the fork that does", has no answer under this option except that
it would not be Stasis.

## Option B: split the hook layer into a separate repository under copyleft

The app stays Apache-2.0 here. The injected component moves to its own
repository under GPL-3.0, and is consumed as a built artefact.

**What it prevents.** One specific thing, and it is narrower than it sounds:
above-ground closed derivatives. A public GitHub repository or a distributed
binary built from GPL-3.0 hook code, shipped without source, is a licence
violation. That gives the owner a takedown claim, which is a low-cost lever and
is the only concrete benefit any copyleft option provides here.

**What it does not prevent.** Underground distribution, which is where cheats
actually live. A subscription cheat sold anonymously through a forum, hosted
offshore, is not going to be resolved by a licence claim from a solo maintainer
with no legal budget. Enforcement is the owner's problem, at the owner's expense,
and the realistic enforcement rate is close to zero.

**Contributor goodwill.** Moderate cost. Two repositories, two licences, and a
boundary every contributor has to reason about before touching the interesting
part of the project. It is the part of the codebase most likely to attract the
contributors worth having, and it is the part now behind a licence question.

**Other costs.** The licence boundary is genuinely murky here and murk is bad for
a one-person project. The GPL-3.0 component is loaded into the *game's* process
rather than into Stasis, and talks to Stasis over IPC, which is a reasonable
argument for two separate works. It is an argument, not a settled question, and
the owner would be relying on it while shipping both halves in one installer.
Apache-2.0 code can be used in a GPL-3.0 work but not the other way round
(https://www.apache.org/licenses/GPL-compatibility.html), so the direction of
travel is one-way and needs to be got right the first time. Also: two CI
pipelines, two release processes, two sets of issues.

Note on which copyleft. GPL-3.0, not AGPL-3.0. AGPL's additional term covers
users interacting with the software over a network, and a desktop overlay is not
a network service, so AGPL would add friction without adding coverage.

## Option C: relicense the whole project to copyleft

Move Stasis, app and engine, to GPL-3.0.

**What it prevents.** The same narrow thing as Option B, applied to the whole
codebase, without the boundary question. Clean, single licence, single repository.

**What it does not prevent.** The same everything as Option B. And one thing
worth stating plainly: relicensing is not retroactive. Every commit already
published under Apache-2.0 stays available under Apache-2.0 forever. Anyone can
fork the last permissive commit. The only thing a relicence protects is what has
not been published yet, which is why the moment before the hook code lands is the
cheapest moment this option will ever have. After that, it protects less every
week.

**Contributor goodwill.** Highest cost of the copyleft options, in a specific
way. `TODO.md` section 1 records the licence as a decided item and `README.md`
explains the reasoning. Reversing a published decision on a project with no
release yet reads as instability, and it is the kind of reversal people remember.
GPL-3.0 does carry an explicit patent grant, so the stated reason for choosing
Apache-2.0 survives the move, and that should be said clearly if this option is
taken.

**Other costs.** The module system is the real problem. `ROADMAP.md` says a
module is meant to be one overlay panel, and the module format becomes a contract
the moment somebody writes against it. Whether a third-party module is a
derivative work of a GPL-3.0 host depends on what a module turns out to be, and
`TODO.md` section 5 has not answered that yet. Choosing GPL-3.0 now constrains a
decision that has not been made, and could deter exactly the third-party
developers the module system exists for. That is a larger cost than the licence
change itself.

## Option D: keep the hook layer closed

Ship the injected component as a signed binary with no published source. Keep the
app open.

**What it prevents.** More than the others, honestly. Not the technique, which is
public, but the specific uplift: a maintained, tested hook layer that a fork can
pick up and build on. This is Overwolf's position and it is not an unreasonable
one (https://www.overwolf.com/our-commitment/).

**What it does not prevent.** Reverse engineering, which for a hook layer is not
a high bar. And nothing at all about cheats built from the many other public
sources.

**Contributor goodwill.** Highest cost of all four, and it is not really about
goodwill. It is about the product's argument.

`README.md` promises no telemetry for every version. `docs/anti-cheat.md`
promises that Stasis does not read game memory, does not send synthetic input,
and does not attempt to evade anti-cheat. `SECURITY.md` invites people to read
roughly 500 lines of Rust and check. Under this option, the component that could
break every one of those promises is the one component nobody can read, and it is
the one that runs inside the user's game process. A closed binary injecting into
your game, from a project that argues you should trust it because you can check,
argues against itself.

It also means the most technically interesting part of the project takes no
contributions, which for a solo maintainer on a multi-year engineering commitment
is a cost with no upside.

**Other costs.** It conflicts with the reproducible-build and verify-the-binary
goals in `TODO.md` section 6, and it would require rewriting the not-a-cheat
sections of three existing documents to remove the word "checkable".

## The mitigation that applies whichever option is chosen

This is licence-independent, costs nothing, and does more than any of the four
options above.

**Keep the injected component narrow, and refuse the generalisations.** The hook
is not the dangerous part. The dangerous part is the toolkit that usually grows
around it. Specifically, Stasis should not build, and should not merge:

- memory read or write helpers of any kind, beyond the hook itself
- back buffer or depth buffer readback exposed to modules
- world-to-screen projection helpers, entity enumeration, or anything shaped like
  a target list
- a general-purpose injection or hooking library, or a refactor that makes the
  hook layer reusable outside Stasis. This is the one a well-meaning contributor
  will propose, framed as good architecture, and it is the single change that
  would most increase the fork hazard.
- any anti-detection, obfuscation, or unhooking capability

The injected DLL should do one thing: attach, draw a surface the host produced,
release, and get out of the way on fault. A fork that wants a cheat then has to
build the interesting half itself, which is most of the work and all of the
game-specific work. `CONTRIBUTING.md` already refuses cheat features; this is the
narrower list of things that are not cheat features but are the parts that make a
fork cheap.

## Recommendation

**This is a recommendation for the owner to accept or reject.**

**Take Option A, and pair it with the narrow-scope mitigation above.**

Four reasons.

**1. The copyleft options buy one lever and it is small.** The only concrete
benefit of B or C is a takedown claim against public closed derivatives. Cheats
are not distributed publicly under their own names, and enforcement costs money
this project does not have. Paying a real price in structure, goodwill, or module
ecosystem uncertainty for a lever that will probably never be pulled is a bad
trade.

**2. The scope discipline is worth more than the licence, and it is free.**
Refusing to build the generic hooking library is the change that actually raises
the cost of forking Stasis into a cheat. It requires no licence change, no second
repository, and no reversal of a published decision. It does require the owner to
say no to a plausible-sounding refactor, repeatedly, which is why it is written
down here.

**3. Option D would cost the project its own argument.** Stasis's pitch is that
you can check. Closing the component that runs inside the game contradicts
`README.md`, `SECURITY.md`, and `docs/anti-cheat.md` at once, and it removes
contributions from the hardest part of the work. The safety it buys is real and
it is not worth that.

**4. Saying it plainly is itself worth something.** A project that publishes "this
code could be forked into a cheat, a licence would not stop that, we chose
openness anyway, and here is what we refuse to build" is in a better position with
a vendor than one that has not thought about it. It is the same move
`README.md` already makes with the telemetry paragraph and `docs/anti-cheat.md`
makes with the residual risk list.

**If the owner wants a licence lever anyway,** Option B is the least bad of the
three alternatives, and the moment to take it is before any hook code is
committed. GPL-3.0, not AGPL-3.0. After the first hook lands under Apache-2.0,
that option loses most of its value permanently, so this is a decision with an
expiry date rather than one that can wait.

**What would change this recommendation.** A named, distributed cheat that
credibly presents itself as a Stasis fork. If that happens, revisit Option C,
while understanding that it protects only versions published after the change.

## Sources

- Overwolf, our commitment, including their stated reason for not open-sourcing: https://www.overwolf.com/our-commitment/
- Apache Software Foundation on GPL compatibility: https://www.apache.org/licenses/GPL-compatibility.html
- GNU licence list, Apache License 2.0 entry: https://www.gnu.org/licenses/license-list.html#apache2
