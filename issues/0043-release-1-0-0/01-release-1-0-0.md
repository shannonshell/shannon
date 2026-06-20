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

## Result

**Result:** Pass.

Shannon `1.0.0` was published through the organization-owned release and
Homebrew paths.

Implementation and release:

- Shannon-owned crate versions were bumped from `0.5.7` to `1.0.0`.
- `dist/shannon.rb` now points at
  `https://github.com/shannonshell/shannon/releases/download/v1.0.0/shannon-1.0.0.tar.gz`.
- The formula source sha256 is
  `94fae37468806eb764d9416350e4dbc641598fdaa57e69306084f0b62b1da5c0`.
- Implementation commit `0dafdc5951c7f204dd96ca1dbba91c30dbb27249` was pushed to
  `shannonshell/shannon`.
- Annotated tag `v1.0.0` resolves to `0dafdc5951c7f204dd96ca1dbba91c30dbb27249`.
- GitHub Release `v1.0.0` exists on `shannonshell/shannon` with
  `shannon-1.0.0.tar.gz`.
- The release asset digest is
  `sha256:94fae37468806eb764d9416350e4dbc641598fdaa57e69306084f0b62b1da5c0`.
- Release notes identify Shannon `1.0.0` as the first stable release, state that
  the embedded Nushell version is `0.113.1`, and include the supported Homebrew
  install commands.

Homebrew publication:

- `shannonshell/homebrew-shannon` formula commit
  `a0b85d3c496d8b7d7fdfba9e47027f7cdf079ed4` updated the source formula to
  `1.0.0`.
- `brew audit --new --strict shannonshell/shannon/shannon` passed.
- Public source install passed with
  `brew install --build-from-source shannonshell/shannon/shannon`.
- Source-install metadata reported:
  - `full_name = shannonshell/shannon/shannon`
  - `tap = shannonshell/shannon`
  - `versions.stable = 1.0.0`
  - source URL =
    `https://github.com/shannonshell/shannon/releases/download/v1.0.0/shannon-1.0.0.tar.gz`
  - `built_as_bottle = false`
  - `poured_from_bottle = false`
- Bottle-mode build passed with
  `brew install --build-bottle shannonshell/shannon/shannon`.
- `brew bottle --json --no-rebuild` failed twice with a Homebrew gzip
  `buffer error`; rerunning with the supported `--skip-relocation` option
  succeeded and still produced an `any_skip_relocation` bottle.
- `shannonshell/homebrew-shannon` formula commit
  `d928cee0c9dcbc7655389369be066ad80746c388` added the `1.0.0` bottle block.
- Tap release `shannon-1.0.0` exists with
  `shannon-1.0.0.arm64_tahoe.bottle.tar.gz`.
- The bottle asset digest is
  `sha256:7386cd77e3c282ed5f7c784568ab833e276f89ced0b186608a70a3bff85af215`.
- Final cold install passed with:

  ```bash
  brew uninstall --force shannon || true
  brew untap ryanxcharles/shannon || true
  brew untap shannonshell/shannon || true
  brew tap shannonshell/shannon
  brew trust shannonshell/shannon
  brew install shannon
  ```

- Final bottle-install metadata reported:
  - `full_name = shannonshell/shannon/shannon`
  - `tap = shannonshell/shannon`
  - `versions.stable = 1.0.0`
  - `versions.bottle = true`
  - source URL =
    `https://github.com/shannonshell/shannon/releases/download/v1.0.0/shannon-1.0.0.tar.gz`
  - bottle root URL =
    `https://github.com/shannonshell/homebrew-shannon/releases/download/shannon-1.0.0`
  - bottle URL =
    `https://github.com/shannonshell/homebrew-shannon/releases/download/shannon-1.0.0/shannon-1.0.0.arm64_tahoe.bottle.tar.gz`
  - `built_as_bottle = true`
  - `poured_from_bottle = true`

Verification:

- `cargo build` passed with the known upstream `nu-command` unfulfilled lint
  expectation warning.
- `cargo test` passed: 10 library tests, 13 binary tests, empty integration
  harness, and empty doctest set.
- `./target/debug/shannon --version` printed `1.0.0 (nushell 0.113.1)`.
- `./target/debug/shannon -c '1 + 2'` printed `3`.
- PTY-backed `expect` smoke with `--no-config-file --no-history` passed:
  - `MODE_START:nu`
  - `NU_OK_RELEASE`
  - `BASH_OK_RELEASE:bash`
  - `MODE_BACK:nu`
  - `ENV_BACK:from_bash_1_0_0`
  - `PWD_BACK:/tmp`
  - `NU_BACK:nu:from_bash_1_0_0:/tmp`
- `scripts/make-source-tarball.sh HEAD /tmp/shannon-release-1.0.0-head` produced
  sha256 `94fae37468806eb764d9416350e4dbc641598fdaa57e69306084f0b62b1da5c0`.
- The source tarball contains `Cargo.toml`, `Cargo.lock`, `nushell/`,
  `reedline/`, and `src/main.rs`, and excludes `dist/shannon.rb`.
- Local Homebrew pre-publication source-build proof passed with a temporary
  `file://` formula URL to the staged `shannon-1.0.0.tar.gz`.
- `brew style shannonshell/shannon/shannon` passed.
- `brew test shannon` passed for both source and bottle installs.
- Installed `shannon --version` printed `1.0.0 (nushell 0.113.1)`.
- Installed `shannon -c '1 + 2'` printed `3`.
- `strings /opt/homebrew/bin/shannon | rg -F '/Users/astrohacker/dev/shannon'`
  produced no output.

Personal-fork checks:

- `brew tap` lists `shannonshell/shannon`, not `ryanxcharles/shannon`.
- `gh release view v1.0.0 --repo ryanxcharles/shannon` reports not found.
- `git ls-remote origin refs/tags/v1.0.0` reports no tag.
- `gh repo view ryanxcharles/homebrew-shannon` reports not found.

## Conclusion

Shannon `1.0.0` is released as the first stable release. The release is
published under `shannonshell/shannon`, the Homebrew formula and bottle are
published under `shannonshell/homebrew-shannon`, and the supported install path
is:

```bash
brew tap shannonshell/shannon
brew trust shannonshell/shannon
brew install shannon
```

The stable behavior bar passed for Nushell mode, non-interactive execution, mode
switching, Bash mode execution, Bash-to-Nushell environment propagation, and
Bash-to-Nushell cwd propagation. The public source-install and bottle-install
paths both passed from the organization-owned tap.

## Completion Review

**Result:** Approved.

Codex reviewed the completed experiment result and found no required fixes
before the result commit. The review accepted the release evidence under
`shannonshell/shannon`, the public Homebrew source and bottle install evidence,
the PTY mode-switch/env/cwd smoke, the release-note checks, and the
personal-fork cleanup checks as sufficient and internally consistent.

The only optional note was procedural: after this result commit, close Issue 43
in a separate step by updating the issue frontmatter, adding the issue-level
conclusion, and regenerating `issues/README.md`.
