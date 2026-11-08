# RFC 0037: Use feature branches into main with manual release bumps

**Status:** Draft

## Summary

Replace Karen's `develop`-to-`main` promotion pipeline with short-lived feature
branches based on `main` and pull requests directly into `main`. Replace
Release Please version PRs with an explicit manual version-bump PR, while
keeping synchronized-version checks and the existing tag-triggered build and
publication behavior.

## Problem

Karen currently requires feature work to land on `develop`, then Release Please
and a promotion workflow move versioned changes into `main`. CI rejects
feature-branch PRs to `main`, and the main ruleset requires the
`Promotion source guard (main)` check. This adds a second integration branch
and makes version changes dependent on an automated release PR. The requested
workflow is feature branch to `main`, with maintainers choosing and committing
version bumps manually.

## Goals

- Start every feature and RFC branch from the latest `origin/main`; open its
  pull request against `main`.
- Keep `main` protected by required pull requests and build/test checks.
- Make release version changes explicit and manual in a dedicated PR to
  `main`; ordinary feature PRs do not bump versions.
- Keep all Cargo, npm, and Tauri versions synchronized and validated.
- Keep release artifact creation, signing, publication, and Homebrew update
  behavior working after a manual version-bump PR merges.
- Retire the Karen repository's `develop` promotion path after reconciling
  unpublished work.

## Non-goals

- Changing release numbering policy or choosing a version for a specific
  release.
- Requiring direct pushes to `main`; pull requests remain mandatory.
- Changing desktop artifacts, signing, updater manifests, or release assets.
- Changing `systembugtj/luban-homebrew` branch conventions; its `develop`
  target belongs to a separate repository.
- Discarding or deleting unpublished commits from local `develop`.

## Design

### Branch and CI flow

Feature, bug-fix, and RFC branches start from current `origin/main` and target
`main`. Update `AGENTS.md` to make that the repository rule. Change
`.github/workflows/ci.yml` to validate pull requests targeting `main` and
remove the job that accepts only `develop` as a promotion source. Keep Linux,
macOS, coverage, and version checks required for `main` pull requests.

Remove `.github/workflows/promote.yml` and the Release Please workflow and its
configuration/manifest. Remove the release-label job from
`.github/workflows/release.yml`, since it only marks automated Release Please
PRs. Preserve `release.yml`'s main-push version validation, tag creation,
signed builds, publication, manual rebuild dispatch, and Homebrew update.

The main GitHub ruleset must stop requiring `Promotion source guard (main)`
while continuing to require pull requests and the existing build/test checks.
Retire the `develop` protection ruleset and remote branch only after checking
that all commits unique to either branch have landed or have another explicit
destination. `main` and `develop` currently have divergent commits; this RFC
must not discard or silently merge them.

### Manual version bump

When preparing a release, create a short-lived branch from `main`, update the
shared version in the root workspace `package.json`, desktop `package.json`,
workspace `Cargo.toml`, Tauri Cargo manifest, Tauri config, and local workspace
package entries in `Cargo.lock`, then open a pull request to `main`. Keep
`pnpm check:versions` as the manifest consistency check and ensure the lockfile
is consistent before merge. Merge of a version-bump PR lets the existing
`release.yml` create the matching tag and publish artifacts. Regular feature
PRs remain version-neutral.

Delete Release Please-only files, including `release-please-config.json` and
`.release-please-manifest.json`; version authority stays in the application
and Cargo manifests, checked by the existing script. Remove the unused
`RELEASE_PLEASE_TOKEN` secret after the workflow is gone.

### Ordered implementation tasks

#### Task 1: Change repository and CI branch contract

Update `AGENTS.md` and `.github/workflows/ci.yml` for feature-branch-to-main
pull requests. Keep main pull request and required test protections.

#### Task 2: Remove develop promotion and automated bump paths

Remove the Release Please and promote workflows, their configuration and
manifest, the main promotion-source guard, and Release Please label handling in
`release.yml`. Preserve release build and publication stages.

#### Task 3: Enforce manual version synchronization

Document the release-bump PR and update version validation as needed so all
manifest and Cargo lock versions must match before release.

#### Task 4: Retire remote develop controls safely

After reconciling unpublished commits, remove the `develop` GitHub ruleset and
remote branch. Remove `Promotion source guard (main)` from the production
ruleset's required checks; retain pull-request and build/test protections.

#### Task 5: Verify direct-to-main delivery and release

Verify feature PR CI, manual version-bump validation, and the existing release
workflow against a new version. Confirm no Release Please or promotion PR is
created and document exact checks.

## Acceptance

- A feature branch opened against `main` runs the required CI checks without a
  promotion-source failure; direct pushes and force pushes remain blocked.
- The `develop`-specific Release Please and promotion workflows no longer run
  or create PRs.
- A manually bumped version PR updates every required manifest and lockfile;
  `pnpm check:versions` and locked Cargo metadata/build validation pass.
- Merging a new version to `main` creates exactly matching `vX.Y.Z` tag and
  runs the existing signed release publication; an unchanged version creates
  no duplicate release.
- Main's active ruleset keeps PR and build/test requirements and no longer
  requires the promotion-source check.
- The Karen `develop` branch and its protection rule are retired only after
  every commit unique to either branch has an explicit disposition.
- `systembugtj/luban-homebrew` update behavior remains unchanged.
