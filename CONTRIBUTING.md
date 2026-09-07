# Contributing to Stasis

Thanks for looking. Read the first section before you write code, because the
project is earlier than most repositories of this shape.

## What exists, and what does not

Stasis is currently the app shell: the window you open outside the game. It
reads your Steam library, lists modules from a folder, and measures its own
memory and CPU. That part works and is wired to real data.

There is no overlay engine. Nothing hooks a game, nothing injects, nothing
renders on top of anything. If you came to work on overlay rendering, the
decisions that come before that code are still open in `TODO.md` section 4, and
the anti-cheat question there has to be settled first. It is not a technical
detail that can be worked around later.

In practice this is a Windows project. `steam.rs` contains macOS and Linux path
detection that has never been run, so treat those paths as unverified rather
than supported.

## Running it

Toolchain the project is developed against:

| Tool | Version |
|---|---|
| Node | 25.9.0 |
| npm | 11.12.1 |
| Rust | 1.95.0 |
| Python | 3.14 (only for `tools/make-icon.py`) |

On Windows you also need the MSVC build tools for the Rust `msvc` toolchain,
and WebView2, which ships with Windows 11 already.

```
npm install
npm run tauri dev      # development, with hot reload
npm run tauri build    # release binary and installers
```

The first `tauri dev` compiles the whole Rust dependency tree and takes a while.
Later runs rebuild only the crate.

## Before you open a pull request

Three commands, all from the repo root:

```
npm run lint     # tsc, clippy with warnings as errors, and a rustfmt check
npm test         # the Rust suite, then the frontend suite
npm run build    # tsc, then the vite build
```

CI runs exactly these on `windows-latest` for every push and pull request, so a
green run locally means a green run there. Tagged releases additionally build
the installers.

The suites split along the Tauri bridge. Rust tests live beside the code they
cover in `src-tauri/src`, and cover the Steam manifest parsing, the library
deduplication, and the module manifest reading. Frontend tests use Vitest with
jsdom, mock `src/lib/api`, and cover the formatters and the view states that are
hard to reach by hand, such as a Steam folder that exists but cannot be listed.

Run the app and use the thing you changed. A build that compiles is not evidence
that a screen works.

## Where things live

- `src/views/` is one file per screen, `src/components/` is the chrome shared
  across them.
- `src/lib/api.ts` is the only boundary to Rust. Every `invoke` goes through it,
  and the product name lives there as one constant.
- `src/styles/tokens.css` holds the palette from `DESIGN.md` as CSS variables.
  Change the values there, not in component styles.
- `src-tauri/src/` is one module per concern. Every command exposed to the
  frontend is registered in `lib.rs`, so that file is the list of what the
  webview can reach.

## Design changes

`DESIGN.md` is the source of design direction, and it is authored by the project
owner. It is not a starting point to be improved by taste, and a pull request
that changes a colour, a radius, a font, or a motion behaviour because it looks
better is the wrong shape. If you think the direction is wrong, open an issue
about the direction.

Two rules follow from that:

- New UI states its reason. Every visual decision in this codebase can be
  explained in one line, and the explanation is usually already in a comment or
  in `DESIGN.md`. Add yours.
- Restraint is the point. The dials are `ENERGY 2 / RHYTHM 2 / MOTION 1`, and
  `MOTION 1` is a hard constraint, not a preference. This is a tool people leave
  open for hours.

Agent instructions, including the design filter the project uses, live in
`CLAUDE.md`.

## What will not be merged

- **Anything that makes this work as cheat software.** Stasis is not a cheat,
  and contributions that add ESP, aimbot assistance, memory reading of other
  processes for gameplay advantage, or anti-cheat evasion will be refused. This
  is stated plainly because an overlay that draws on top of a game sits close
  enough to that category that leaving it to inference would be a mistake. It is
  also the risk most likely to end the project, since users getting banned is
  not something it would recover from.
- **Telemetry, analytics, ads, or an account requirement.** The README commits
  to their absence for every version. A dependency that phones home counts.
- **Dead controls.** A button that does nothing, a nav item pointing at a screen
  that does not exist, or a toggle with no engine behind it. The comment at the
  top of `src/components/Rail.tsx` explains why the rail has three destinations
  instead of six: the other three would have been links to nothing.
- **Fabricated content.** No sample games, no placeholder modules that look
  installed, no statistics without a source. Every screen in this app is wired
  to something real, and the empty states say they are empty.

## Reporting bugs and vulnerabilities

Security problems go through the process in `SECURITY.md`, not a public issue.

For ordinary bugs, the useful ones say which GPU and driver version, which game
and launcher, and what the footprint bar read at the time.

## Licence

Stasis is Apache-2.0. Under section 5 of that licence, a contribution you
submit for inclusion is licensed under the same terms unless you say otherwise.
There is no separate CLA to sign.
