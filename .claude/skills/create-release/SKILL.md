---
name: create-release
description: Create a new octamedic release for a given version (e.g. "0.0.2" or "v0.0.2"). Bumps the crate versions, writes a detailed changelog entry in docs/CHANGELOG.md covering everything since the previous release, commits, tags and pushes so the release workflow builds and publishes the CLI and GUI binaries. Use when asked to "create a release", "release vX.Y.Z", "cut a release" or "publish a new version".
argument-hint: <version>
---

# Create release

Make release `v<version>` of octamedic. The argument is the version; accept it with or without a
leading `v` and normalize: the tag is `vX.Y.Z`, the crate version is `X.Y.Z`. If no version was
given, ask for one.

Pushing a tag publishes a GitHub release, so nothing is pushed until the user has approved the
changelog (step 4).

## 1. Check the state

- `git fetch --tags origin`, then make sure the tag doesn't already exist locally or on the
  remote. If it does, stop and ask; never move or overwrite a release tag unless the user says
  so explicitly.
- Must be on `main` and level with or ahead of `origin/main`. If behind, stop and ask.
- The version must be higher than the previous release tag (semver). If not, stop and ask.
- If `git status` shows uncommitted changes, ask whether they belong in the release. Never
  commit them without that answer; never stash or discard them.

## 2. Collect what changed

Previous release: `git describe --tags --abbrev=0 --match 'v*'` (if there is none, this is the
first release and the range is the whole history).

Read all of this, not just commit messages: commit messages in this repo are often short or
vague ("uiredo", "voicetest"), so the diff is the source of truth.

- `git log --reverse --format='%h %s%n%b' <prev>..HEAD`
- `git diff --stat <prev>..HEAD`
- `git diff <prev>..HEAD -- octamed octamed_cli octamedic_core octamedic_gui README.md Cargo.toml`
  and read the hunks. Open the changed files where the diff alone doesn't make the behavior
  clear.
- The README and `TODO.md` diffs for items that were ticked off.
- `Cargo.toml` dependency changes (workspace root and each crate).

The workspace ships two programs: the CLI (`octamed_cli`) and the GUI (`octamedic_gui`), built
on `octamedic_core` (engine/playback) and `octamed` (OctaMED module parsing/writing). Work out
what changed for someone using either program: new features, changed behavior, commands and
keys, module format support (reading/writing MMD files), fixes. Say which program a change
affects when it's only one of them. Internal refactors only get a line if they change
something users notice (performance, binary size, stability, playback quality).

Check every claim against the code. Something whose implementation is `todo!()` or otherwise
unfinished is not a feature; leave it out, or list it under known issues if a user could
run into it.

## 3. Write the changelog entry

`docs/CHANGELOG.md`, newest release first. Create it if missing with a
`# Changelog` heading. The release workflow takes the release notes from this file by finding
the line `## vX.Y.Z` and reading up to the next `## ` heading, so the entry heading must be
exactly:

```
## vX.Y.Z - YYYY-MM-DD
```

Use today's date. Under it, use `###` sections, only the ones that have entries, in this order:

- `### Added`: new features, commands, keys, options
- `### Changed`: behavior that works differently now
- `### Fixed`: bugs fixed
- `### Removed`
- `### Known issues`

Write for users, in plain words: what they can do now and how (the CLI command and its syntax,
where in the GUI), not which function changed. One bullet per change, more detail for bigger
features: a short example (like a CLI invocation) is good where it helps. Don't mention commit
hashes, internal type names or file paths, except docs a user would open.

If the previous release has no entry in the file, don't backfill it unless asked.

## 4. Get approval

Show the user the full changelog entry and summarize what will happen: version bump, the
commit, the tag, and that pushing publishes a release. Wait for approval and apply any edits
they ask for.

## 5. Bump, commit, tag, push

- Set `version = "X.Y.Z"` under `[package]` in every workspace member's `Cargo.toml`
  (`octamed`, `octamed_cli`, `octamedic_core`, `octamedic_gui`) so they stay in step. Run a
  plain `cargo build` so `Cargo.lock` picks up the new versions (CI builds with `--locked`,
  which fails on a stale lock), then
  `cargo build --release --locked -p octamed_cli -p octamedic_gui` to make sure both release
  binaries build. If it doesn't build, stop and report; don't tag broken code.
- Stage the member `Cargo.toml`s, `Cargo.lock`, `docs/CHANGELOG.md` and whatever the user
  agreed to include.
- Commit with the single-line message `vX.Y.Z` and nothing else: no body, no co-author
  trailer. The author is the repo's configured git user.
- `git tag vX.Y.Z`, then `git push origin main` and `git push origin vX.Y.Z`.

## 6. Report

Say what was pushed (commit hash, tag) and link the run and the release:
`https://github.com/TheExiledCat/octamedic/actions` and
`https://github.com/TheExiledCat/octamedic/releases/tag/vX.Y.Z`. The release has a CLI zip and
a GUI zip per target. `gh` is not installed, so you can't watch the run; tell the user to check
it.
