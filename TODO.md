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

- [ ] **Anti-cheat strategy, before any injection code is written.** This is
      the single largest risk to the project and it is not primarily technical.
      Overwolf's real moat is that it is known to EAC, BattlEye, and publishers,
      so its overlay does not get users banned. An unknown overlay that hooks
      `Present` looks exactly like an ESP cheat to an anti-cheat driver, and
      being open source cuts both ways: auditable, but also trivially forkable
      into a cheat, which is precisely why vendors distrust it. Decide early
      whether to pursue vendor relationships, restrict to games without kernel
      anti-cheat, or use only officially sanctioned overlay APIs. Users getting
      banned would end the project.
- [ ] Choose the rendering path: D3D11/D3D12/Vulkan/OpenGL hooking, versus a
      transparent click-through always-on-top window. The second is far safer
      with anti-cheat and much worse for latency and exclusive fullscreen.
- [ ] Overlay process model: in-process hook, or a separate process compositing
      over the game. Separate is safer and slower.
- [ ] Global hotkey registration, and a toggle that is guaranteed to release
      input back to the game.
- [ ] Exclusive fullscreen versus borderless behaviour, per game.

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
- [ ] Enable and disable per module, and per game. The rail already shows
      counts; the state has nowhere to live yet.
- [ ] Installation flow. Folder drop works today and is honest; a registry or
      index is a much larger commitment.

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

- [ ] Launchers beyond Steam: Epic, GOG, Xbox/Game Pass, Battle.net, EA,
      Ubisoft, and plain executables. The scanner is currently Steam-only, and
      `ScanResult` will need to become per-source rather than one Steam shape.
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
- [x] Structured logging with a log file the user can find and attach to a bug
      report. `tauri-plugin-log` writes `logs/stasis.log` inside the app data
      folder, next to `settings.json`, capped at one megabyte with the old file
      dropped when it fills. Tauri's own log directory was not used: it is under
      LocalAppData while everything else this app writes is under Roaming, and
      one folder to point a user at is worth more than the platform default.
      Every failure path in `steam.rs`, `modules.rs`, `settings.rs`, and
      `footprint.rs` writes a line, and the footprint sampler writes its failure
      once rather than every two seconds forever. Paths are redacted to `~`
      before they reach the file, since the point of it is a public bug report.
      The bug form no longer says there is nothing to attach.
- [x] Settings now validate the Steam folder before a scan is attempted, on
      arrival and on blur. Four answers, not two: empty is auto-detection and
      says nothing, and a missing path, a file, and a real folder with no
      `steamapps` each get their own sentence. `check_steam_path` and the
      scanner share `holds_steam_library`, so the field and the scan cannot
      disagree about the same folder. The message is tied to the input with
      `aria-describedby` and `aria-invalid` rather than being red text. Saving a
      path that does not resolve is still allowed: an unplugged external drive
      is a real reason to keep one.
- [ ] A light theme, if it is ever wanted. Dark is currently the only theme and
      `DESIGN.md` gives the reason. If light is added, both modes have to be
      fully correct, not one plus an afterthought.

## 9. Things to write down before contributors arrive

- [x] An explicit statement that this is not a cheat and that contributions
      enabling cheating will be refused. Said in `CONTRIBUTING.md` under "What
      will not be merged", and in `SECURITY.md` for security reports.
- [ ] Decide whether the not-a-cheat line also belongs in the README. It is the
      first file most people read, and right now the statement only exists one
      click deeper.
- [x] Keep `DESIGN.md` authoritative for visual decisions, and require new UI
      to state its reason. Both rules are in `CONTRIBUTING.md` under "Design
      changes".
- [ ] A roadmap that separates "shell" from "engine" so contributors do not
      assume the overlay already works.
