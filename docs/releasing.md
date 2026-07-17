# Releasing Audis

Audis finds new versions by reading the GitHub releases of
[`neura-spheres/Audis`](https://github.com/neura-spheres/Audis). Everything below
follows from that one fact: the tag _is_ the contract.

## Tags

| Kind   | Tag             | GitHub flag | Who sees it       |
| ------ | --------------- | ----------- | ----------------- |
| Stable | `v1.2.3`        | Latest      | Everyone          |
| Beta   | `v1.2.3-beta.1` | Pre-release | Beta channel only |

The tag is semver with a `v` in front. That is not decoration: the app parses it
and compares versions with semver rules, so the ordering falls out for free.

- `1.2.0-beta.1` < `1.2.0` — a beta tester is offered the finished release when
  it lands, rather than being stranded on the pre-release.
- `1.2.0-beta.2` > `1.2.0-beta.1` — betas order among themselves.
- `1.3.0-beta.1` > `1.2.0` — a beta of a later version still counts as newer.

Tick **"Set as a pre-release"** on GitHub for any `-beta.N` tag. The app treats a
release as a beta if _either_ the tag or GitHub's flag says so, so the two
disagreeing fails safe rather than pushing a beta to everyone.

A tag that is not semver (`nightly`, say) is ignored by the update check, not an
error. Draft releases are ignored too.

## How updating works

`check_for_updates` reads the releases list, picks the newest one the user's
channel allows, and compares it against the running version. About then shows the
version and notes with **Update and restart**, which downloads the installer,
verifies it, runs it, and restarts into the new version.

Discovery goes through the GitHub API rather than a fixed endpoint because GitHub
has no "latest pre-release" address: `releases/latest` skips pre-releases, so a
static endpoint could never serve the beta channel. Audis finds the right release
for the channel, then points the updater at _that_ release's `latest.json`.

Nothing is installed unless its signature matches the public key compiled into
the binary. That check is the whole basis for downloading and running an
installer automatically: without it, anyone able to answer for GitHub could hand
the app an executable of their choosing. A release with no `latest.json` — one
cut before the updater existed — falls back to a link to the release page.

## The signing key

The updater's trust rests on one keypair.

- **Public half**: `plugins.updater.pubkey` in `tauri.conf.json`. Committed; it
  is meant to be public.
- **Private half**: generated with `pnpm tauri signer generate`, kept **outside
  the repository**, and set as the `TAURI_SIGNING_PRIVATE_KEY` repository secret
  (with `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`) for CI.

Two things follow, and both are permanent:

- **If it leaks**, whoever has it can ship an update every Audis install will
  accept and run. Treat it like a code-signing certificate.
- **If it is lost**, no existing install can ever auto-update again — they only
  trust the key baked into the binary they already have. Changing the key means
  every user reinstalls by hand. Back it up somewhere durable.

## Releasing

Push a tag. `.github/workflows/release.yml` builds, signs, and publishes it,
deriving the pre-release flag from the tag so the two cannot disagree:

```
git tag v1.2.3 && git push origin v1.2.3          # stable
git tag v1.2.3-beta.1 && git push origin v1.2.3-beta.1   # beta
```

Set the version in the workspace `Cargo.toml` and `apps/desktop/tauri.conf.json`
first, and make them match the tag: the app compares releases against its own
compiled-in `CARGO_PKG_VERSION`.

To build locally instead, `./scripts/build.ps1 -Bundle`, with
`TAURI_SIGNING_PRIVATE_KEY` set if the artifacts need to be installable. Close
Audis first: Windows locks a running `.exe` and the build fails with "Access is
denied (os error 5)".
