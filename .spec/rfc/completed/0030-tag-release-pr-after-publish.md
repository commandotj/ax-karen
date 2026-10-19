# RFC 0030: Mark the merged release PR as tagged after publishing (child of 0001)

**Status:** Implemented

**Parent:** [0001](0001-karen-system-resource-app.md)

## Summary

After the Release workflow publishes a GitHub Release from `main`, flip the merged Release Please version PR from `autorelease: pending` to `autorelease: tagged`, so Release Please can open the next version PR.

## Problem

`release-please.yml` and its config set `skip-github-release: true`; the Release workflow, not Release Please, creates the tag and release. Release Please therefore never retags its own PR. PR #12 stayed `autorelease: pending` after v0.12.0, and every later push to `develop` ended with `There are untagged, merged release PRs outstanding - aborting`, so no version PR was opened and nothing was promoted. The label was flipped by hand to unblock (PR #18 then opened); without a fix the next release repeats the failure.

## Goals

- A successful publish of a release triggered by a push to `main` relabels every merged PR on `develop` that still carries `autorelease: pending` to `autorelease: tagged`.
- The label is created if missing.
- A failure in this step is visible but does not block the Homebrew job or undo the published release.

## Non-goals

- No change to `promote.yml`; it must still see `autorelease: pending` when the version PR merges, which happens before publishing.
- Not run for `workflow_dispatch` rebuilds of an old tag, which must not tag a newer version PR waiting for promotion.
- Open version PRs are never touched.

## Design

Add a job `tag-release-pr` to `.github/workflows/release.yml` that needs `prepare` and `publish`, runs only when `needs.prepare.outputs.proceed == 'true'` and the event is `push`, and uses the workflow token (`pull-requests: write` is already granted). It runs `gh label create ... || true`, lists merged PRs with `--base develop --label "autorelease: pending"`, and replaces the label on each. Being a separate job keeps `homebrew-tap-pr` independent of it.

## Acceptance

- The workflow file parses as YAML and `actionlint` reports nothing new, if available.
- The new job is gated on `push`, `publish` and `proceed`, and does not appear in the dependency chain of `homebrew-tap-pr`.
- The next real release flips the merged version PR to `tagged` and the following push to `develop` opens or updates the next version PR. This cannot be exercised before a release, so it is verified on the next release run.
