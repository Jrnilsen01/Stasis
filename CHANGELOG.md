# Changelog

Notable changes to Stasis, in the format of
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Unreleased

Nothing has been released yet. The current state is the app shell described in
`README.md`: it reads your Steam library, lists modules from a folder, and
measures its own memory and CPU. There is no overlay engine behind it.

The first entry here will be written when there is a first tag.

## Versioning

Stasis follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html), with
one thing worth saying plainly while the major version is 0: the product this is
meant to become does not exist yet. Until the overlay engine lands and the module
format is settled, a minor bump can change anything, including the shape of
`module.json`. Treat 0.x as the period where the contracts are still being
written, not a stable product with small revisions.

Once a module format ships, its own schema version is a separate promise from
this one, and breaking it is a breaking change regardless of what the app version
does. That schema does not exist yet either; `TODO.md` section 5 tracks it.

## Where the version lives

One place: `version` in `src-tauri/Cargo.toml`.

Tauri reads it from there because `tauri.conf.json` deliberately has no `version`
key, which is the documented fallback. `package.json` has no `version` either,
since it is a private package that publishes nothing and a second number would
only be one to forget.

To release, bump that one line. Nothing else needs to match it.
