# Experiment 1: Publish Stable 1.0.0 Release

## Description

Publish Shannon `1.0.0` as the first stable release through the
organization-owned release and Homebrew paths.

This experiment promotes the already-verified `0.5.7` packaging path to a stable
`1.0.0` release. It should keep the scope tight: version bump,
documentation/release-note updates, deterministic source asset, public GitHub
Release, tap formula update, bottle if supported, and cold install verification.
It should not introduce new shell behavior unless verification finds a release
blocking defect.

Before implementation, this design must be reviewed by another AI agent. All
required findings must be fixed, the review result must be recorded in this
file, and the approved plan must be committed separately from implementation.

The release is stable if these criteria hold:

- Shannon starts reliably in Nushell mode.
- Non-interactive Nushell command execution works.
- Bash mode executes through the persistent Bash subprocess.
- Bash mode preserves cwd and exported environment changes back into Nushell.
- The binary reports `1.0.0 (nushell 0.113.1)`.
- The public Homebrew source and bottle install paths work from the
  `shannonshell/shannon` tap.
- Documentation and release notes name the organization-owned repositories and
  supported install command.

## Changes

Repo changes:

- bump Shannon-owned crate versions from `0.5.7` to `1.0.0`;
- update root and vendored lockfiles for the Shannon-owned packages;
- update `dist/shannon.rb` to point at
  `https://github.com/shannonshell/shannon/releases/download/v1.0.0/shannon-1.0.0.tar.gz`;
- update the formula sha256 after generating the deterministic release tarball;
- update `dist/shannon.rb` tests to expect Shannon `1.0.0` and Nushell
  `0.113.1`;
- update README/release-facing documentation if any text still implies a
  pre-1.0.0 release;
- record this experiment's result and conclusion.

Publication changes:

- verify `git remote get-url upstream` points at `shannonshell/shannon`;
- verify `git remote get-url origin` is not used for publication;
- verify no existing `v1.0.0` tag or release exists under
  `shannonshell/shannon`;
- verify no existing `v1.0.0` tag or release exists under
  `ryanxcharles/shannon`;
- verify no `ryanxcharles/homebrew-shannon` tap repository exists;
- commit the implementation locally before tagging;
- push the implementation commit explicitly with `git push upstream main`;
- create annotated tag `v1.0.0` on the implementation commit;
- push the tag explicitly with `git push upstream v1.0.0`;
- build `shannon-1.0.0.tar.gz` from the tag with
  `scripts/make-source-tarball.sh`;
- create GitHub Release `v1.0.0` under `shannonshell/shannon`;
- upload `shannon-1.0.0.tar.gz` to that release;
- update `shannonshell/homebrew-shannon` with the `1.0.0` formula;
- build and publish a bottle if supported on this machine;
- verify a cold public install through `brew tap shannonshell/shannon`.

Release notes should state:

- Shannon `1.0.0` is the first stable release;
- embedded Nushell version is `0.113.1`;
- supported install path is:

  ```bash
  brew tap shannonshell/shannon
  brew trust shannonshell/shannon
  brew install shannon
  ```

## Verification

Pre-publication checks:

1. Confirm authority and clean state:
   - `git status --short --branch` shows only intentional implementation changes
     before commit, then a clean worktree after commit;
   - `git remote get-url upstream` points at `shannonshell/shannon`;
   - `git remote get-url origin` points at the personal fork and is not used for
     publication;
   - `gh repo view shannonshell/shannon --json viewerPermission` reports
     `ADMIN`;
   - `gh repo view shannonshell/homebrew-shannon --json viewerPermission`
     reports `ADMIN`;
   - `gh release view v1.0.0 --repo shannonshell/shannon` reports not found;
   - `git ls-remote upstream refs/tags/v1.0.0` reports no tag;
   - `gh release view v1.0.0 --repo ryanxcharles/shannon` reports not found;
   - `git ls-remote origin refs/tags/v1.0.0` reports no tag;
   - `gh repo view ryanxcharles/homebrew-shannon` reports not found.
2. Local product checks:
   - `cargo build`;
   - `cargo test`;
   - `./target/debug/shannon --version` prints `1.0.0 (nushell 0.113.1)`;
   - `./target/debug/shannon -c '1 + 2'` prints `3`;
   - PTY-backed mode smoke uses `--no-config-file --no-history` and verifies:
     - Shannon starts in `nu` mode with `$env.SHANNON_MODE == "nu"`;
     - the mode switch path works through `__shannon_switch` or Shift+Tab;
     - Shannon enters `bash` mode with `$env.SHANNON_MODE == "bash"`;
     - bash command execution works;
     - exported bash environment changes propagate back into Nushell;
     - bash cwd changes propagate back into Nushell;
     - switching back to `nu` works and Nushell sees the propagated env/cwd.
3. Source archive checks:
   - deterministic tarball generation produces a stable sha256;
   - source tarball includes `Cargo.toml`, `Cargo.lock`, `nushell/`,
     `reedline/`, and `src/main.rs`;
   - source tarball excludes `dist/shannon.rb`.

Homebrew pre-publication proof:

- use a local `shannonshell/shannon` tap formula with a `file://` URL to the
  staged `shannon-1.0.0.tar.gz`;
- `brew style shannonshell/shannon/shannon` passes;
- `brew install --build-from-source shannonshell/shannon/shannon` succeeds;
- `brew test shannon` passes;
- installed `shannon --version` prints `1.0.0 (nushell 0.113.1)`;
- installed `shannon -c '1 + 2'` prints `3`;
- installed binary strings do not expose this checkout path.

Publication checks:

- `git ls-remote upstream refs/heads/main refs/tags/v1.0.0 refs/tags/v1.0.0^{}`
  shows the expected implementation commit and tag under `shannonshell/shannon`;
- `gh release view v1.0.0 --repo shannonshell/shannon` shows the uploaded
  `shannon-1.0.0.tar.gz` asset with the formula's sha256;
- `gh release view v1.0.0 --repo shannonshell/shannon --json body` shows release
  notes that include:
  - `1.0.0` as the first stable release;
  - embedded Nushell `0.113.1`;
  - the `brew tap shannonshell/shannon`, `brew trust shannonshell/shannon`, and
    `brew install shannon` commands;
- `gh repo view shannonshell/homebrew-shannon` shows a public repo;
- the tap formula URL points at `github.com/shannonshell/shannon`, not
  `github.com/ryanxcharles/shannon`;
- `brew audit --new --strict shannonshell/shannon/shannon` passes, or any
  third-party tap warnings are recorded and justified.

Public source install checks:

```bash
brew uninstall --force shannon || true
brew tap shannonshell/shannon
brew install --build-from-source shannonshell/shannon/shannon
brew info --json=v2 shannon
brew test shannon
shannon --version
shannon -c '1 + 2'
```

The source-install `brew info --json=v2 shannon` result must prove:

- `full_name = shannonshell/shannon/shannon`;
- `tap = shannonshell/shannon`;
- `versions.stable = 1.0.0`;
- source URL is
  `https://github.com/shannonshell/shannon/releases/download/v1.0.0/shannon-1.0.0.tar.gz`;
- `built_as_bottle = false` and `poured_from_bottle = false`.

Cold public install checks:

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

The `brew info --json=v2 shannon` result must prove:

- `full_name = shannonshell/shannon/shannon`;
- `tap = shannonshell/shannon`;
- `versions.stable = 1.0.0`;
- source URL is under `https://github.com/shannonshell/shannon/releases`;
- bottle root URL is under
  `https://github.com/shannonshell/homebrew-shannon/releases` if a bottle is
  published;
- `built_as_bottle = true` and `poured_from_bottle = true` if a bottle is
  published.

Cleanup and regression checks:

- `brew tap` does not list `ryanxcharles/shannon`;
- `gh release view v1.0.0 --repo ryanxcharles/shannon` reports not found;
- `git ls-remote origin refs/tags/v1.0.0` reports no tag;
- no `v1.0.0` release or tag is created under `ryanxcharles/shannon`;
- `gh repo view ryanxcharles/homebrew-shannon` reports not found;
- `strings /opt/homebrew/bin/shannon | rg -F '/Users/astrohacker/dev/shannon'`
  produces no output.

Pass criteria:

- Shannon `1.0.0` is released under `shannonshell/shannon`.
- Shannon is installable through
  `brew tap shannonshell/shannon && brew trust shannonshell/shannon && brew install shannon`.
- The installed package reports `1.0.0 (nushell 0.113.1)`.
- Source install and bottle install both work if bottling is supported.
- Documentation and release notes state the supported install path.
- No release or Homebrew artifact is published under the personal fork.

Fail criteria:

- The `1.0.0` binary fails the stable behavior smoke tests.
- The deterministic source archive cannot be built or verified.
- The public release, formula, source install, or bottle install fails.
- Any artifact is accidentally published under the personal fork.

## Design Review

**Result:** Approved.

Codex reviewed the initial design and required four concrete fixes: add public
source-install verification after publication, make the stable behavior smoke
test explicit about PTY mode switching and env/cwd propagation, verify release
note contents, and add personal-fork preflight/final checks. The design was
updated for all required findings and now also states the design-review and
plan-commit gate before implementation.

The follow-up review approved the design with no required findings remaining.
