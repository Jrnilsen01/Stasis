## What this changes

<!-- What it does and why. If it closes an issue, say "Closes #123". -->

## How you tested it

<!--
Which screens or commands you actually exercised, and on what. There is no CI
yet, so this section is the only evidence a reviewer gets.
-->

## Checks

Run from the repo root, then `src-tauri` for the Rust ones. See `CONTRIBUTING.md`.

- [ ] `npm run build` passes
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] I ran the app and used the thing I changed

## If this touches the UI

- [ ] The change follows `DESIGN.md` rather than personal taste
- [ ] Every new visual decision has its reason written down, in a comment or in `DESIGN.md`
- [ ] No new motion beyond hover and focus states (`MOTION 1` is a hard constraint)
- [ ] Colours come from `src/styles/tokens.css`, not from values typed into a component
- [ ] Empty, loading, and error states are handled, not just the happy path

## Confirmations

- [ ] No dead controls: every button, link, and toggle I added does something real
- [ ] No sample data, placeholder games, or fabricated content
- [ ] No telemetry, analytics, ads, or account requirement, including via a new dependency
- [ ] This does not make Stasis usable as cheat software
