# TODO

Ordered by what blocks what, not by size. Items marked **[decision]** need the
owner to choose before anyone can act on them.

The honest state of the project: the app shell works and is verified, there is
no overlay engine, no test suite, no CI, and no git history. Everything below
follows from that.

---

## 1. Decisions needed before going public

- [x] **[decision] Confirm the name.** Decided: **Stasis**. Applied across the
      code, config, README, and DESIGN.md.
- [x] **Clear the name.** Cleared by the owner. The known collision with
      STASIS, the 2015 adventure game by The Brotherhood that sells on Steam,
      was raised and accepted.
- [x] **[decision] Pick a licence.** Decided: **Apache-2.0**, for the explicit
      patent grant. `LICENSE` is the verbatim upstream text, and `package.json`
      and `Cargo.toml` both carry the SPDX id.
- [x] Fill the copyright holder in the `LICENSE` appendix. Set to
      `Copyright 2026 Jrnilsen01`.
- [x] **[decision] Repo owner and name.** Decided by creating it: the personal
      account, at `Jrnilsen01/Stasis`. Move it to an organisation later if
      anyone else ever needs commit rights.
- [x] **[decision] Telemetry stance.** Decided: no telemetry, no ads, no
      account, committed to in the README as a promise for every version rather
      than a description of this one. Nothing in the current code contradicts
      it; keep it that way, and treat any future analytics dependency as a
      change to that section first.

## 2. Before the first public commit

- [x] `git init` and the first commit. Branch `main`, commit `d14af87`, 69
      files. Author identity is set repo-locally to the GitHub noreply address
      so no personal email lands in public history; the global git config was
      left untouched.
- [x] Add the chosen `LICENSE` file, and a licence line in `README.md`.
- [x] Extend `.gitignore`. Correction to an earlier note here: `src-tauri/target/`
      and `gen/schemas` were already covered, by `src-tauri/.gitignore` rather
      than the root one. The root rules are redundant but harmless, and more
      discoverable. Genuinely added: `app-icon-sizes.png`, and `__pycache__/`
      plus `*.pyc`, which was catching real bytecode staged from `tools/`.
- [x] `.gitattributes` with `* text=auto` and explicit binary rules for the
      icons and fonts, so line endings do not vary by whoever committed last.
- [x] `public/` removed. The premise of this item was slightly off: there was
      no `publicDir` line in `vite.config.ts` to drop, so the folder was only
      Vite's default convention directory sitting empty. Nothing referenced it,
      and `npm run build` is clean without it. Vite picks the convention back up
      on its own if a `public/` is ever added again.
- [x] `CONTRIBUTING.md` written. Covers the shell-versus-engine state, the
      toolchain table, the local checks that stand in for the missing CI, the
      `DESIGN.md` authority rule, and what will not be merged.
- [x] `SECURITY.md` written. It routes reports through GitHub's private
      vulnerability reporting rather than an email address, and documents the
      real current surface: `reveal_path`, the Steam manifest parsing, the
      `module.json` read, and the settings file.
- [ ] Enable private vulnerability reporting in the repo's Settings, Security
      tab. `SECURITY.md` tells reporters to use it, so it has to be switched on
      or that instruction is a dead end.
- [x] `CODE_OF_CONDUCT.md`. Contributor Covenant 2.1, verbatim apart from the
      enforcement contact, which names @Jrnilsen01 through GitHub.
- [ ] Give the Code of Conduct a private reporting channel. It currently routes
      through GitHub with no private option, which the file says out loud. A
      conduct report that has to be filed in public is a report that often does
      not get filed. Worth fixing before the project has a community rather
      than after.
- [x] Issue and PR templates, in `.github/`. Three issue forms (bug, proposal,
      question) plus `PULL_REQUEST_TEMPLATE.md`. The bug form requires Windows
      version, GPU and driver, and the commit built from, and asks for game and
      launcher where a title is involved. Blank issues are disabled so the
      structured fields are not bypassed.
- [x] Added a `contact_links` entry to `.github/ISSUE_TEMPLATE/config.yml`
      pointing at the advisory page for Jrnilsen01/Stasis. It only becomes a
      working route once private vulnerability reporting is switched on.
- [x] `CHANGELOG.md` and a versioning policy. Solved with a single source of
      truth rather than a sync script: `version` now lives only in
      `src-tauri/Cargo.toml`. The key was removed from `tauri.conf.json`, which
      makes Tauri fall back to Cargo.toml by documented behaviour, and from
      `package.json`, which is private and publishes nothing. Verified end to
      end: the built binary reports ProductVersion 0.1.0.
- [ ] The Tauri bundle identifier changed to `com.joaki.stasis`, which moves the
      app data directory. Any settings written under the old identifier are
      orphaned. Harmless before release, worth noting so it is not diagnosed
      twice.
- [x] `Cargo.toml` authors set to `["Jrnilsen01"]`.

## 3. Correctness debt worth paying first

The VDF parsing is the highest-value test target in the codebase: it already
shipped one real bug, where the registry spelling and the manifest spelling of
the same Steam folder were treated as two libraries and every game was listed
twice.

- [x] Rust unit tests for `steam.rs`. 19 tests covering `vdf_values` against a
      real manifest sample, the case-and-separator dedupe in `library_folders`,
      the override-is-authoritative behaviour in `candidate_roots`, and
      `games_in_library`. The dedupe test was mutation-checked: reverting the
      canonicalisation makes it fail with "the same folder in two spellings was
      counted twice", so it genuinely guards the bug rather than passing by
      accident.
- [x] Rust tests for `modules.rs` manifest parsing. 8 tests, including the
      malformed `module.json` surfacing as an error rather than a silent skip,
      and that `path` comes from the folder rather than from the manifest, so a
      module cannot claim to live somewhere it does not. Needed a small
      refactor: `read_modules(&Path)` split out of `list_modules`, because the
      `AppHandle` there only supplies a path and cannot be built in a test.
- [x] Frontend tests for `format.ts`. 13 tests including all three named
      boundaries: 1023 versus 1024 bytes, 102297 versus 102400 for the 99.9 to
      100 decimal switch, and null, undefined, NaN, and Infinity handling.
      Vitest with jsdom is now set up; the config lives in `vite.config.ts`.
- [x] Exercise the three UI states never yet triggered. All three now have
      component tests that mock `src/lib/api`: Steam unreadable, Steam with zero
      games, and a modules folder read failure. Each asserts the distinction
      that matters, in particular that an unreadable folder reports a null count
      rather than zero, since zero would render as a successful scan that found
      nothing.
- [x] CI on Windows, in `.github/workflows/ci.yml`. The `check` job runs
      `npm run lint` (tsc, clippy, fmt), both test suites, and the build on
      every push and pull request. The `bundle` job runs `tauri build` on `v*`
      tags and uploads the installers. Windows only on purpose: building on
      macOS or Linux would give a green tick to path code nobody has run.
- [x] Added `lint` and `test` scripts to `package.json`. `npm test` runs the
      Rust suite; `npm run lint` runs `tsc --noEmit`, clippy, and a fmt check.
- [x] `npm run lint` passes. Both clippy findings became `sort_by_key`, and
      rustfmt was run across the crate: 3 files, 29 insertions, 12 deletions,
      and zero comment lines touched, so the wrapped prose in the source is as
      it was. `CONTRIBUTING.md` now documents checks the repo actually meets,
      which means CI will be green on its first run rather than red.

## 4. The product: the overlay engine

This is the entire thing, and none of it exists yet.

- [ ] **Anti-cheat strategy, before any injection code is written.** Still the
      single largest risk, and still not primarily technical.

      One correction, recorded rather than quietly edited. This file used to say
      Overwolf's moat is that it is known to EAC, BattlEye and publishers.
      Research found nothing Overwolf publishes that names an anti-cheat vendor:
      their guarantees are about publisher terms of service and per-app
      curation, and their own API reference advertises `requestGameInjection()`
      across D3D9, D3D11, D3D12 and Vulkan. They hook like anyone else.

      What the evidence does support: Riot states there is no allow list for
      Vanguard, BattlEye supports non-cheat overlays unless a developer objects,
      and the one well-documented ban case (Overplus in Dota 2, 2024) turned on
      what a module displayed rather than how it was drawn. Bans came from
      content, not technique, which moves that risk to section 5.

      **Decided: hook the graphics API.** Chosen by the owner over the
      non-injecting transparent window that the research recommended, and over
      sanctioned-APIs-only. The trade is accepted knowingly: feature parity with
      the incumbent and correct behaviour in every presentation mode, against a
      per-API renderer to maintain, crash liability inside someone else's game,
      and the fork hazard below.

      The research is unverified, and three quotes in it are load-bearing. They
      still need checking before any of this is published as the project's
      public position, but they no longer block the build.
- [ ] **Measure how many target titles actually present via exclusive
      fullscreen.** Less decisive now that hooking is chosen, since hooking
      handles every presentation mode. Still worth an afternoon: it sizes how
      much of the benefit the harder path is actually buying.
- [ ] **The fork hazard is now live, and needs an answer.** An Apache-2.0
      repository containing working `Present` hooks for several graphics APIs is
      a cheat scaffold, and Overwolf says publicly that this is why their
      technology stays closed. The not-a-cheat statements in `README.md`,
      `CONTRIBUTING.md` and `SECURITY.md` are now load-bearing rather than
      preventative. Options, none of them comfortable: accept it and say so
      plainly, keep the hook layer in a separate repository under a copyleft
      licence, or revisit the section 1 licence decision. Deciding not to decide
      is the same as choosing the first.
- [ ] **Riot titles are out of scope, by Riot's own statement.** No allow list
      for Vanguard, and their line is injected code. Valorant and League cannot
      be supported on this path. Write that into the supported-games policy
      rather than letting users discover it.
- [ ] **Plan for the patch day that breaks injection.** Valve's CS:GO Trusted
      Mode required Authenticode signing and blocked DLLs that interfere with
      the game, and injecting overlays stopped working overnight. Users blamed
      the overlays. Decide in advance how Stasis detects it has been blocked,
      what it tells the user, and how fast a fix can ship. This is a
      when-not-if item on the chosen path.
- [ ] **Authenticode signing is now a build-time dependency, not just a
      distribution nicety.** Section 6's code signing item moves earlier: an
      unsigned injected DLL is refused outright by some anti-cheat
      configurations, so this gates the engine rather than the installer.
- [ ] **Crash isolation inside someone else's process.** A fault in the hook
      crashes the user's game, and the crash dump names Stasis. Decide the
      guard: what is wrapped, what is never done on the render thread, and how
      the overlay disables itself after a fault rather than crashing the same
      game twice.
- [ ] **A supported-games list, published.** Overwolf maintains one rather than
      claiming universal support, because per-title quirks are unavoidable on
      this path. Better to ship the honest list than to have users find the
      gaps.
- [ ] **Write to BattlEye and Epic describing the architecture, then publish the
      answers including silence.** A positive response would be the strongest
      asset the project could have. No response documents that the relationship
      route was not available, which is worth publishing too.
- [x] Choose the rendering path. Decided: hooking. What remains is which APIs
      and in what order.
- [ ] **Pick the first graphics API and ship only that.** D3D11 is the widest
      installed base and the best documented; D3D12 and Vulkan are more work per
      title. Shipping one API that works beats five that half work, and the
      supported-games list makes the limit honest.
- [ ] **The injection mechanism itself.** How the DLL gets into the target, and
      the failure path when it cannot. This is the part that most resembles a
      cheat to an observer, so it deserves the clearest code and comments in the
      repository.
- [ ] Overlay process model: in-process hook, or a separate process compositing
      over the game. Separate is safer and slower.
- [ ] Global hotkey registration, and a toggle that is guaranteed to release
      input back to the game.
- [ ] Exclusive fullscreen versus borderless behaviour, per game.

The five decisions above gate the code. Everything below holds whichever way
they land, so it can be specified now rather than discovered during the build.

- [ ] **Game detection.** Knowing a game started, which process it is, and which
      of its windows to draw over. Process enumeration, window class matching,
      and launcher integration fail differently, and attaching to the wrong
      window is worse than attaching to none.
- [ ] **The attach and detach lifecycle.** Game start, alt-tab, minimise,
      resolution change, moving to another monitor, exit, and crash. Each is a
      transition the overlay has to survive. The crash case matters most: the
      overlay must never be the reason a game dies.
- [ ] **The input state machine.** Click-through by default, input captured only
      while the overlay is focused, and a release path that holds even if the
      overlay itself has hung. The failure here is a player who cannot control
      their game, which is worse than a crash because it looks like the game's
      fault.
- [ ] **A performance budget, stated as a number.** "Lighter than the incumbent"
      is the product's whole argument and is currently an adjective. Decide the
      frame time and memory the overlay is allowed to cost, and how that is
      measured, before there is code to measure. The footprint bar already does
      this for the shell; the engine is the part that will actually cost
      something.
- [ ] **Multi-monitor and mixed DPI.** Which display the game is on, what
      happens when it moves, and how the overlay scales when two monitors
      disagree about DPI.
- [ ] **Honest degradation on an unsupported renderer.** A game on an unhandled
      graphics API should say so. An overlay that silently never appears is the
      worst version of this, because the user cannot tell it from a bug.
- [ ] **HDR and colour space.** Overlays routinely break HDR output or render
      washed out over it. Worth knowing which before the palette is committed to
      a compositing path.
- [ ] **Whether the overlay appears in captures.** OBS, ShadowPlay, and Game Bar
      will either see it or not, depending on how it is drawn. Both answers are
      defensible. Arriving at one by accident is not.
- [ ] **A way to turn it off without launching the app.** If a hook misbehaves
      mid-session, the route out cannot run through the thing that is
      misbehaving. A file, a flag, or a safe mode.
- [ ] **How the engine gets tested at all.** Hooking cannot be unit tested the
      way `steam.rs` can. A small harness that presents frames like a game does
      is probably the only honest answer, and it is worth building before the
      engine rather than after.

## 5. The module system

Third parties will write modules the moment the repo is public, so the format
becomes a contract earlier than expected.

- [ ] Specify and **version** the `module.json` schema. It currently has four
      untyped fields and no version key, so there is no way to evolve it.
- [ ] Decide what a module actually is: static HTML/JS in a webview, a WASM
      module, or a native plugin. This determines the entire security model.
- [ ] Module sandboxing and a permission model. A module that can read the
      screen and the filesystem is a keylogger waiting to happen, and "we are
      not adware" dies the first time a module exfiltrates something.

      Now on the critical path for the anti-cheat decision rather than
      downstream of it. The Overplus case in section 4 says bans follow from
      what a module displays, so a module showing an enemy ability timer gets
      users banned no matter how carefully the engine draws it. A content policy
      is part of this, not a separate concern.
- [ ] Enable and disable per module, and per game. The rail already shows
      counts; the state has nowhere to live yet.
- [ ] Installation flow. Folder drop works today and is honest; a registry or
      index is a much larger commitment.
- [ ] **Module lifecycle.** Load, unload, enable, disable, and reload without
      restarting the app or the game.
- [ ] **A per-module resource budget.** One heavy module must not be able to
      spend the performance argument the whole product rests on.
- [ ] **Crash isolation.** A module that fails should take down itself and
      nothing else, least of all the game.
- [ ] **A compatibility policy.** What happens when a module written against an
      older schema meets a newer host, and the reverse. Left implicit now, this
      is the part that hurts most later.
- [ ] **Module development without a game.** Whoever writes a module needs to
      see it render without launching a title and alt-tabbing after every edit.

## 6. Distribution and trust

- [ ] **Code signing.** The installers are unsigned, so Windows SmartScreen
      will warn every user. For a product whose pitch is trustworthiness this
      matters more than usual. An OV certificate is the cheap path; EV clears
      SmartScreen immediately and costs more.
- [ ] Auto-update. Tauri's updater needs a signing key and a hosted manifest,
      and the key handling should be settled before the first release.
- [ ] Release workflow: tag, build, sign, attach installers, publish notes.
- [ ] Reproducible-ish builds so a user can verify a binary matches the source.
      This is worth real effort for an overlay people are asked to trust.

## 7. Coverage

Launchers beyond Steam, ranked by effort against value. Three of these were
verified against a real install during research; the rest rest on prior art and
are marked as such in the spec.

- [ ] **Epic.** A directory of JSON `.item` manifests carrying id, name,
      `InstallSize`, install path and launch executable already. Cheapest of the
      six and high reach.
- [ ] **Battle.net.** The uninstall registry alone covered every installed game
      on the test machine, with `UninstallString` carrying the uid. Strongest
      audience overlap for an overlay. Note that its two data sources are
      incomplete in opposite directions, so the union keyed on uid is the only
      correct answer.
- [ ] **GOG.** Registry sub-keys plus `goggame-<id>.info`, and the only source
      with a real local `last_played`.
- [ ] **EA.** Reuses the Battle.net uninstall scanner with a different
      predicate. Two hard rules: do not decrypt the `IS` file, which is
      plausibly circumvention and unnecessary anyway, and report
      `size_on_disk` as `None`, because the size EA records is present and
      wrong by a factor of eighty in one measured case.
- [ ] **Xbox and Game Pass.** Highest value below Epic and the most distinct
      work: a `.GamingRoot` binary parse, XML, and per-drive partial failure.
- [ ] **Ubisoft.** Registry gives paths cheaply, but names need a heuristic
      parser whose own reference implementation admits to guesswork. Smallest
      audience, highest maintenance. Last for a reason.
- [ ] **Plain executables**, for anything none of the above knows about.
- [ ] **Redesign `ScanResult` as per-source.** Keep the three states, and add
      `Disabled` and `Unsupported` so neither is mistaken for `NotFound`: "you
      have no Epic" and "you asked me not to look" are different sentences.
      Partial failure belongs inside a successful scan as warnings, so one
      readable Xbox drive and one denied stays a good scan with a caveat. Never
      merge duplicates: two installs are two directories with two executables,
      and the overlay attaches to a process one specific launcher started.
- [ ] **Add `launch_executable` to `Game`.** Free from Epic, GOG and Xbox, and
      it is the field the overlay engine will need to match a running process.
- [ ] **Say "not recorded" where a launcher does not record it.** `last_played`
      is absent for five of the six new sources. An empty cell reads as "never
      played", which is a different and false statement, and sorting by last
      played would put almost everything in one bucket.
- [ ] macOS and Linux path detection in `steam.rs` is written but has never
      been run. Either test it or mark the project Windows-only for now and say
      so in the README.
- [ ] Steam library changes are only picked up on a manual rescan. A file
      watcher on `steamapps` would be cheap and better.

## 8. UI and accessibility debt

- [ ] Text scaling. Tauri disables zoom hotkeys by default, so users cannot
      resize text in-app; only OS DPI scaling applies. Either enable
      `zoomHotkeysEnabled` and verify the layout at 200%, or document that DPI
      scaling is the supported route.
- [ ] Screen reader pass. Roles and focus order look right and have never been
      tested with NVDA or Narrator.
- [ ] Structured logging with a log file the user can find and attach to a bug
      report. There is currently no logging at all.
- [ ] Settings currently lack validation: the Steam folder accepts any string
      and only fails at scan time. Inline validation would be kinder.
- [ ] **Contrast over unpredictable content.** Every ratio in `DESIGN.md` is
      measured against a known surface. An overlay drawn on live gameplay has no
      guaranteed background, so the same palette that passes AA in the shell can
      fail over a bright scene. The overlay needs its own contrast strategy, a
      scrim or a backing surface, rather than inheriting the shell's numbers.
- [ ] A light theme, if it is ever wanted. Dark is currently the only theme and
      `DESIGN.md` gives the reason. If light is added, both modes have to be
      fully correct, not one plus an afterthought.

## 9. Things to write down before contributors arrive

- [x] An explicit statement that this is not a cheat and that contributions
      enabling cheating will be refused. Said in `CONTRIBUTING.md` under "What
      will not be merged", and in `SECURITY.md` for security reports.
- [x] The not-a-cheat line is now in the README, as its own section next to the
      telemetry commitment, and it points at `CONTRIBUTING.md` and
      `SECURITY.md` for the contribution and report cases.
- [x] Keep `DESIGN.md` authoritative for visual decisions, and require new UI
      to state its reason. Both rules are in `CONTRIBUTING.md` under "Design
      changes".
- [x] `ROADMAP.md` separates the shell from the engine: what works and is
      tested, what is gated on the anti-cheat decision, what the module system
      waits on, and what runs in parallel. It carries no dates, and says why.
      Linked from the README paragraph that establishes the current state.
