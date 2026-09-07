# Vendor letters

Drafts. Nothing here has been sent, and nothing here should be sent by anyone
other than the owner. Fill the blanks, read both letters end to end, and send
them yourself.

`TODO.md` section 4 lists this as an item: write to BattlEye and Epic describing
the architecture, then publish the answers including silence.

## What to expect

The research behind these drafts could not establish that either vendor operates
a third-party overlay compatibility programme. No published policy addressed to
overlay developers, no submission process, no allowlist application, and no
precedent of a small open-source project obtaining an arrangement. That is
recorded as unverified rather than as an absence, because absence of a public
page is not proof there is nothing behind it.

The likely outcome is no reply. That outcome is worth publishing. "We described
the architecture to both vendors on *[date]* and received no response" is a
factual sentence that tells a user something real, and it is a better answer than
implying a relationship exists.

Three rules for whatever comes back:

- **Do not ask to be allowlisted.** Riot's third-party FAQ states there is
  absolutely no allow list for Vanguard and that Developer Relations cannot carve
  out loopholes or perform secret handshakes
  (https://www.riotgames.com/en/DevRel/vanguard-faq). Whether or not the other
  vendors take the same position, asking for an exemption frames Stasis as
  something that needs one. The question is whether the architecture is a
  concern, not whether an exception can be made.
- **Publish the reply in full, or not at all.** A partial quote from an
  anti-cheat vendor is worse than silence, because it will be read as an
  endorsement. If a reply arrives marked confidential, publish the fact that it
  arrived and nothing else.
- **A reply is not permission.** Even a positive answer describes the vendor's
  current posture and not the publisher's, and BattlEye's own FAQ says non-cheat
  overlays are generally supported unless the game developer wants otherwise
  (https://www.battleye.com/support/faq/). `docs/anti-cheat.md` should not gain a
  safety claim on the strength of an email.

## Where to send them

**BattlEye.** The contact form at https://www.battleye.com/contact/ is the only
published route found. No developer or partner channel was found.

**Epic, for Easy Anti-Cheat.** Epic's anti-cheat documentation lives under
https://dev.epicgames.com/docs/epic-online-services and developer contact is
routed through the Epic Online Services Developer Portal. The exact route for a
third-party software vendor, as opposed to a game developer integrating the SDK,
could not be established and should be treated as unverified. If the portal
requires an organisation account with a game attached, say so in the eventual
write-up, because that would itself answer the question about whether the
relationship route is available.

Send them separately. Do not copy one vendor on the other.

---

## Letter 1: BattlEye

**Subject:** Third-party overlay architecture, request for guidance

> To BattlEye,
>
> I maintain Stasis, an open-source game overlay host for Windows, licensed
> Apache-2.0. It is at https://github.com/Jrnilsen01/Stasis. It has not been
> released and the overlay engine is not yet written, which is why I am writing
> now rather than after shipping.
>
> The architecture, stated precisely:
>
> Stasis injects a DLL into a running game process and hooks the graphics API's
> presentation call, `IDXGISwapChain::Present` on Direct3D 11, in order to draw a
> user interface into the back buffer before the frame is presented.
>
> It does not read or scan game memory. It does not write game memory beyond the
> hook itself. It does not send synthetic input to the game. It does not modify
> game files. It does not load a kernel driver. It does not request elevation, and
> it will not attach to a game running with higher privileges. It makes no attempt
> to hide the injected module, obfuscate its signature, unhook other software, or
> evade detection of any kind. It opens no network socket.
>
> The overlay draws user-supplied panels. What those panels may display is
> constrained by a permission model that is still being designed, and the project
> refuses contributions that would show a player information the game has not
> already given them.
>
> Your FAQ states that you generally ban only for actual cheats, that non-cheat
> overlays are generally supported unless the game developer wants otherwise, and
> that you may kick rather than ban for specific programs. I have read it and I am
> not asking you to restate it.
>
> My questions:
>
> 1. Does the architecture above raise a concern from your side, on the technique
>    alone, independent of what the overlay draws?
> 2. Is there anything in that list you would want changed or added before a tool
>    like this attaches to a BattlEye-protected title?
> 3. Is there a process by which a third-party overlay developer can raise
>    compatibility questions with you, or report a suspected interaction? I could
>    not find one published.
>
> I intend to publish your reply, or the fact that there was no reply, in the
> project's public documentation. If you would prefer the reply not be published,
> say so and I will publish only that a reply was received.
>
> [YOUR NAME]
> [YOUR EMAIL]
> [DATE]

---

## Letter 2: Epic Games, Easy Anti-Cheat

**Subject:** Third-party overlay architecture, Easy Anti-Cheat compatibility
question

> To the Easy Anti-Cheat team,
>
> I maintain Stasis, an open-source game overlay host for Windows, licensed
> Apache-2.0, at https://github.com/Jrnilsen01/Stasis. It has not been released
> and the overlay engine is not yet written. I am writing before building rather
> than after.
>
> The architecture, stated precisely:
>
> Stasis injects a DLL into a running game process and hooks the graphics API's
> presentation call, `IDXGISwapChain::Present` on Direct3D 11, in order to draw a
> user interface into the back buffer before the frame is presented.
>
> It does not read or scan game memory. It does not write game memory beyond the
> hook itself. It does not send synthetic input to the game. It does not modify
> game files. It does not load a kernel driver. It does not request elevation. It
> makes no attempt to hide the injected module, obfuscate its signature, unhook
> other software, or evade detection. It opens no network socket. The injected
> component will be Authenticode signed before release.
>
> Your documentation on integrating the anti-cheat states that software relying on
> undocumented operating system behaviours, using self-modifying code, or patching
> system libraries is very likely to conflict with the anti-cheat service. Hooking
> the presentation call is patching a system library, so I am taking that sentence
> as describing this architecture and asking about it directly rather than assuming
> it does not apply.
>
> My questions:
>
> 1. Is a presentation-call hook of this shape a known conflict source for Easy
>    Anti-Cheat, or is the concern in that documentation aimed at a narrower class
>    of software?
> 2. Does Authenticode signing of the injected component change your answer?
> 3. Is there a channel through which a third-party software vendor, as distinct
>    from a game developer integrating the SDK, can raise a compatibility question
>    or report a suspected interaction? I could not find one published, and if
>    there is none I would like to be able to say that accurately.
>
> I intend to publish your reply, or the fact that there was no reply, in the
> project's public documentation. If you would prefer the reply not be published,
> say so and I will publish only that a reply was received.
>
> [YOUR NAME]
> [YOUR EMAIL]
> [DATE]

---

## After sending

Record, in this file, for each vendor: the date sent, the route used, and the
outcome. Update `docs/anti-cheat.md` only if the outcome changes a fact stated
there. A non-response changes nothing except that the sentence "no vendor has
told us this is acceptable" gains a date.

| Vendor | Route | Sent | Outcome |
|---|---|---|---|
| BattlEye | https://www.battleye.com/contact/ | not sent | |
| Epic, Easy Anti-Cheat | EOS Developer Portal, route unverified | not sent | |

## Sources

- BattlEye FAQ: https://www.battleye.com/support/faq/
- BattlEye contact: https://www.battleye.com/contact/
- Epic Online Services, using the anti-cheat interfaces: https://dev.epicgames.com/docs/epic-online-services/trust-and-safety/anti-cheat-interfaces/using-anti-cheat
- Epic Online Services documentation: https://dev.epicgames.com/docs/epic-online-services
- Riot Vanguard third-party FAQ: https://www.riotgames.com/en/DevRel/vanguard-faq
