+++
status = "closed"
opened = "2026-06-20"
closed = "2026-06-20"
+++

# Issue 43: Release 1.0.0

## Goal

Prepare and publish Shannon `1.0.0` as the first stable release, with clear
release criteria, verified install paths, and a public release record that users
can trust.

## Background

Shannon is now installable through the organization-owned Homebrew tap:

```bash
brew tap shannonshell/shannon
brew trust shannonshell/shannon
brew install shannon
```

Issue 41 upgraded the embedded Nushell dependency set to Nushell `0.113.1`.
Issue 42 published Shannon `0.5.7` through `shannonshell/shannon` and
`shannonshell/homebrew-shannon`, including a verified source release asset and
an `arm64_tahoe` bottle.

A `1.0.0` release should not be only a version bump. It should define what
"stable" means for Shannon, confirm that the supported workflows are reliable,
and avoid carrying forward any accidental release or tap state from the
pre-1.0.0 packaging work.

## Analysis

The release should treat the current architecture as the intended stable shape:

- Shannon remains a poly-shell built on the vendored Nushell binary source.
- Nushell mode remains the default and should preserve stock Nushell behavior.
- Bash mode remains a persistent Bash subprocess with environment and cwd
  propagation back into Nushell.
- Shift+Tab remains the mode switch between `nu` and `bash`.
- Distribution goes through GitHub Releases and the
  `shannonshell/homebrew-shannon` tap.

The release process should verify both the product behavior and the publication
surface. That means testing the local binary, release tarball generation,
Homebrew source builds, Homebrew bottles, and public install commands. It should
also check that documentation names the right organization-owned repositories
and does not refer users to personal forks or obsolete install paths.

## Requirements

The completed `1.0.0` release should provide:

- Shannon-owned crate versions bumped to `1.0.0`.
- A GitHub tag `v1.0.0` in `shannonshell/shannon`.
- A GitHub Release `v1.0.0` in `shannonshell/shannon`.
- A deterministic `shannon-1.0.0.tar.gz` source asset attached to that release.
- A `dist/shannon.rb` formula pointing at the `v1.0.0` source asset with the
  correct sha256.
- An updated formula in `shannonshell/homebrew-shannon`.
- A Homebrew bottle if supported on the release machine.
- README/install documentation that points users to `shannonshell/shannon` and
  `shannonshell/homebrew-shannon`.
- Release notes that state the embedded Nushell version and the supported
  install path.

The verification bar should include:

- `cargo build`
- `cargo test`
- `shannon --version` reporting `1.0.0 (nushell 0.113.1)`
- non-interactive Nushell command execution
- Bash mode smoke testing, including env and cwd propagation
- deterministic source tarball generation and sha256 verification
- `brew style shannonshell/shannon/shannon`
- `brew audit --new --strict shannonshell/shannon/shannon`
- Homebrew source install from the public tap
- Homebrew bottle install from the public tap, if a bottle is published
- `brew test shannon`
- confirmation that no installed binary or package metadata exposes the local
  checkout path

## Open Questions

- What exact behavioral criteria should define Shannon `1.0.0` as stable?
- Should `1.0.0` include additional documentation, completions, shell
  integration, or migration notes beyond the current README?
- Should the release publish bottles for more than the current release machine's
  platform?
- Should any pre-1.0.0 Homebrew artifacts be retained, superseded, or annotated
  in release notes?

## Experiments

- [Experiment 1: Publish stable 1.0.0 release](01-release-1-0-0.md) — **Pass**

## Conclusion

Shannon `1.0.0` is released as the first stable release.

The release is published under the Shannon organization:

- `shannonshell/shannon` tag `v1.0.0`
- `shannonshell/shannon` GitHub Release `v1.0.0`
- source asset `shannon-1.0.0.tar.gz`
- `shannonshell/homebrew-shannon` formula and `arm64_tahoe` bottle

The supported install path is:

```bash
brew tap shannonshell/shannon
brew trust shannonshell/shannon
brew install shannon
```

Verification covered local build/test, non-interactive Nushell execution, a
PTY-backed nu/bash mode-switch smoke with env and cwd propagation, deterministic
source tarball generation, public Homebrew source install, public Homebrew
bottle install, `brew style`, `brew audit --new --strict`, `brew test`, release
note checks, path-leak checks, and personal-fork absence checks.

The final installed package reports `1.0.0 (nushell 0.113.1)` and Homebrew
metadata confirms it installs from `shannonshell/shannon` and pours the bottle
from `shannonshell/homebrew-shannon`.
