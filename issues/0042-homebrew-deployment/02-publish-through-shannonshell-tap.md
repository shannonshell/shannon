# Experiment 2: Publish Through Shannonshell Tap

## Description

Publish Shannon through the correct organization-owned Homebrew tap:
`shannonshell/homebrew-shannon`.

Experiment 1 proved the formula shape locally but failed because the publication
target was the personal `ryanxcharles` account. This experiment starts from that
failure record, cleans up or supersedes the accidental personal artifacts, bumps
Shannon to a fresh patch version, and publishes through the `shannonshell`
organization.

The release version for this corrected publication is `0.5.7`.

Why not reuse `0.5.6`: the accidental personal fork already has tag `v0.5.6` and
GitHub Release `v0.5.6`. Even if those are removed, reusing the same version
would make local caches, tap history, and audit records ambiguous. A fresh patch
release gives the correct organization-owned Homebrew path an unambiguous
artifact line.

## Changes

Repo changes:

- bump Shannon-owned crate versions from `0.5.6` to `0.5.7`;
- update `dist/shannon.rb` to point at
  `https://github.com/shannonshell/shannon/releases/download/v0.5.7/shannon-0.5.7.tar.gz`;
- update `README.md` if needed so the Homebrew path remains:

  ```bash
  brew tap shannonshell/shannon
  brew trust shannonshell/shannon
  brew install shannon
  ```

- record this experiment's result and conclusion.

Accidental personal-account cleanup:

- inventory the accidental personal-account artifacts before deletion, including
  repository URL, release URL, tag SHA, release asset name, and release asset
  sha256;
- stop for explicit confirmation after the inventory and before any destructive
  remote deletion;
- delete the accidental `ryanxcharles/homebrew-shannon` repository;
- delete the accidental `v0.5.6` GitHub Release from `ryanxcharles/shannon`;
- delete the accidental `v0.5.6` tag from `ryanxcharles/shannon` and from the
  local checkout;
- untap `ryanxcharles/shannon` locally;
- uninstall any locally installed `shannon` formula from the accidental tap
  before the public cold install verification.

Correct publication:

- verify the git remotes before publishing:
  - `git remote get-url upstream` must point at `shannonshell/shannon`;
  - `git remote get-url origin` must not be used for publication in this
    experiment;
- push the corrected Shannon commits explicitly to the org remote with
  `git push upstream main`;
- create tag `v0.5.7` on the corrected implementation commit and push it to the
  org remote with `git push upstream v0.5.7`;
- verify the pushed commit and tag with
  `git ls-remote upstream refs/heads/main refs/tags/v0.5.7`;
- build `shannon-0.5.7.tar.gz` from that tag with
  `scripts/make-source-tarball.sh`;
- create GitHub Release `v0.5.7` on `shannonshell/shannon` and upload the
  tarball;
- create public repository `shannonshell/homebrew-shannon`;
- publish `Formula/shannon.rb` and a tap README there;
- build and publish a bottle if Homebrew supports it on this machine;
- verify cold install from `brew tap shannonshell/shannon`.

## Verification

Pre-publication checks:

1. Confirm authority and clean state:
   - `gh repo view shannonshell/shannon --json viewerPermission` reports
     `ADMIN`;
   - `git remote get-url upstream` points at `shannonshell/shannon`;
   - `git remote get-url origin` points at the personal fork and is not used for
     publication;
   - `gh repo view shannonshell/homebrew-shannon` reports not found before
     creation;
   - `gh release view v0.5.7 --repo shannonshell/shannon` reports not found;
   - `git ls-remote upstream refs/tags/v0.5.7` reports no tag;
   - the worktree is clean after the implementation commit.
2. Local formula proof before publication:
   - source tarball includes `Cargo.toml`, `Cargo.lock`, `nushell/`,
     `reedline/`, and `src/main.rs`;
   - local tap formula builds with `brew install --build-from-source`;
   - `brew style` passes for the local tap formula;
   - `brew test shannon` passes;
   - installed `/opt/homebrew/bin/shannon --version` prints
     `0.5.7 (nushell 0.113.1)`;
   - installed `/opt/homebrew/bin/shannon -c '1 + 2'` prints `3`;
   - installed binary strings do not expose this checkout path.

Publication checks:

- `gh release view v0.5.7 --repo shannonshell/shannon` shows the uploaded
  `shannon-0.5.7.tar.gz` asset with the formula's sha256.
- `git ls-remote upstream refs/heads/main refs/tags/v0.5.7` shows the expected
  implementation commit and tag under `shannonshell/shannon`.
- `gh repo view shannonshell/homebrew-shannon` shows a public repo.
- The tap formula URL points at `github.com/shannonshell/shannon`, not
  `github.com/ryanxcharles/shannon`.
- `brew style shannonshell/shannon/shannon` passes.
- `brew audit --new --strict shannonshell/shannon/shannon` passes, or any
  third-party tap warnings are recorded and justified.
- A cold install works:

  ```bash
  brew uninstall --force shannon || true
  brew untap ryanxcharles/shannon || true
  brew untap shannonshell/shannon || true
  brew tap shannonshell/shannon
  brew trust shannonshell/shannon
  brew install shannon
  brew info --json=v2 shannon
  brew test shannon
  shannon --version
  shannon -c '1 + 2'
  ```

Cleanup checks:

- `gh repo view ryanxcharles/homebrew-shannon` reports not found after cleanup.
- `gh release view v0.5.6 --repo ryanxcharles/shannon` reports not found after
  cleanup.
- `git ls-remote origin refs/tags/v0.5.6` reports no tag after cleanup.
- `git tag --list v0.5.6` reports no local tag after cleanup.
- `brew tap` does not list `ryanxcharles/shannon`.

Pass criteria:

- Shannon is installable through
  `brew tap shannonshell/shannon && brew trust shannonshell/shannon && brew install shannon`.
- The installed package reports `0.5.7 (nushell 0.113.1)`.
- The formula and release assets live under `shannonshell`, not `ryanxcharles`.
- The accidental personal tap/release artifacts are removed or proven absent.

Fail criteria:

- We cannot authenticate with sufficient permission to publish
  `shannonshell/homebrew-shannon`.
- The `shannonshell` release or tap publish fails.
- The public tap install cannot build or pour Shannon.
- Personal-account artifacts remain active and could confuse users.

## Design Review

**Result:** Approved.

Codex reviewed the design and initially required three fixes: make the cold
install proof uninstall any existing Shannon package and verify its source, add
explicit `upstream` remote and push guards for `shannonshell/shannon`, and make
personal-account cleanup auditable with an inventory plus explicit confirmation
before destructive deletion. The design was updated for all three findings and
the issue spine was updated to reflect the corrected `0.5.7` release line.

The follow-up review approved the design with no required findings remaining. It
left one optional implementation note: when running
`brew info --json=v2 shannon`, record the exact tap/source metadata proving the
installed formula came from `shannonshell/shannon`.

## Result

**Result:** Pass.

Shannon was published through the organization-owned Homebrew path:

- `shannonshell/shannon` main was pushed to commit
  `e60b663aba9d68456ebaba5aa2c1912b2edd54a2`.
- Tag `v0.5.7` was pushed to `shannonshell/shannon`; the annotated tag resolves
  to `e60b663aba9d68456ebaba5aa2c1912b2edd54a2`.
- GitHub Release `v0.5.7` exists on `shannonshell/shannon` with
  `shannon-0.5.7.tar.gz`.
- The release asset digest is
  `sha256:9ee34faa76b8a60530f7360d172b1094f02a93e022a7d29decf635d90f9b995c`,
  matching `dist/shannon.rb`.
- Public tap `shannonshell/homebrew-shannon` exists with formula
  `Formula/shannon.rb`.
- Tap release `shannon-0.5.7` exists with bottle asset
  `shannon-0.5.7.arm64_tahoe.bottle.tar.gz`.
- The bottle digest is
  `sha256:55961cc18def8b261e7613785c6a150c95878a4fb852b3724af9b30c221eccf1`.

The accidental personal-account artifacts were cleaned up:

- `gh repo view ryanxcharles/homebrew-shannon` reports not found.
- `gh release view v0.5.6 --repo ryanxcharles/shannon` reports not found.
- `git ls-remote origin refs/tags/v0.5.6` reports no tag.
- `git tag --list v0.5.6` reports no local tag.
- `brew tap` lists `shannonshell/shannon`, not `ryanxcharles/shannon`.

Verification:

- `cargo test` passed.
- `scripts/make-source-tarball.sh HEAD /tmp/shannon-release-0.5.7-head` produced
  sha256 `9ee34faa76b8a60530f7360d172b1094f02a93e022a7d29decf635d90f9b995c`.
- The source tarball contains `Cargo.toml`, `Cargo.lock`, `nushell/`,
  `reedline/`, and `src/main.rs`.
- Local source-build proof passed through a temporary `shannonshell/shannon` tap
  using the staged tarball.
- `brew style shannonshell/shannon/shannon` passed.
- `brew audit --new --strict shannonshell/shannon/shannon` passed.
- Public source install passed with
  `brew install --build-from-source shannonshell/shannon/shannon`.
- Bottle build passed with
  `brew install --build-bottle shannonshell/shannon/shannon` and
  `brew bottle --json --no-rebuild`.
- Public bottle install passed with `brew install shannonshell/shannon/shannon`;
  Homebrew reported `built_as_bottle: true` and `poured_from_bottle: true`.
- `brew info --json=v2 shannon` reported:
  - `full_name = shannonshell/shannon/shannon`
  - `tap = shannonshell/shannon`
  - `versions.stable = 0.5.7`
  - `urls.stable.url = https://github.com/shannonshell/shannon/releases/download/v0.5.7/shannon-0.5.7.tar.gz`
  - `bottle.stable.root_url = https://github.com/shannonshell/homebrew-shannon/releases/download/shannon-0.5.7`
  - `installed[0].built_as_bottle = true`
  - `installed[0].poured_from_bottle = true`
- `brew test shannon` passed.
- `shannon --version` printed `0.5.7 (nushell 0.113.1)`.
- `shannon -c '1 + 2'` printed `3`.
- `strings /opt/homebrew/bin/shannon | rg -F '/Users/astrohacker/dev/shannon'`
  produced no output.

## Conclusion

The corrected Homebrew deployment is live under the Shannon organization. Users
can install Shannon with:

```bash
brew tap shannonshell/shannon
brew trust shannonshell/shannon
brew install shannon
```

The package installs Shannon `0.5.7`, embeds Nushell `0.113.1`, and has both a
verified source-build path and a working `arm64_tahoe` bottle. The failed
personal-account Homebrew artifacts from Experiment 1 were removed.

## Completion Review

**Result:** Approved.

Codex reviewed the completed experiment result and found no required fixes for
the result commit. The review accepted the `shannonshell` release, tap, source
install, bottle install, and `ryanxcharles` cleanup evidence as sufficient and
internally consistent.

The only optional note was procedural: do not close Issue 42 until after this
experiment result is committed, then add the issue-level conclusion, update the
issue frontmatter to closed, and rebuild `issues/README.md`.
