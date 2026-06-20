+++
status = "closed"
opened = "2026-06-20"
closed = "2026-06-20"
+++

# Issue 42: Homebrew deployment

## Goal

Make Shannon installable through Homebrew as a normal command-line package,
using a dedicated tap and a formula-based release path.

The intended user path should be:

```bash
brew tap shannonshell/shannon
brew trust shannonshell/shannon
brew install shannon
```

The tap must be owned by the Shannon organization:
`shannonshell/homebrew-shannon`. Do not publish a Shannon Homebrew tap under
`ryanxcharles/homebrew-shannon`.

## Background

Shannon is a Rust command-line shell built from this repository, with vendored
Nushell and Reedline source trees. Unlike TermSurf, Shannon does not ship a GUI
`.app` bundle or a large prebuilt Chromium runtime, so a Homebrew cask is not
the right packaging model.

Two nearby projects provide useful patterns:

- TermSurf uses a cask in `termsurf/homebrew-termsurf` because it installs a
  prebuilt macOS app bundle, CLI, Roamium, and Chromium runtime resources from a
  GitHub Release tarball.
- NuTorch uses a formula in `nutorch/homebrew-nutorch` because it is a CLI
  package. Its main repo keeps the formula source of truth in `dist/nutorch.rb`;
  the tap repo receives a copy with release URLs, sha256 pins, and bottle
  metadata. NuTorch also documents Homebrew 6.0's third-party tap trust gate:
  `brew tap`, `brew trust`, then `brew install`.

Shannon should follow the NuTorch-shaped model: a formula, not a cask.

## Analysis

The Homebrew package should install the `shannon` binary and any support files
needed for a normal shell install. The first implementation should be simple and
auditable:

- keep the formula source of truth in this repository, likely `dist/shannon.rb`;
- publish the formula through the separate Homebrew tap repository
  `shannonshell/homebrew-shannon`;
- point the formula at a stable, hash-pinned GitHub Release source tarball,
  preferably an uploaded release asset rather than GitHub's regenerable
  `/archive/` tarballs;
- use `depends_on "rust" => :build` for source builds;
- build with `cargo build --release`;
- install the release binary as `bin/shannon`;
- add a `test do` block that proves the installed binary starts and reports the
  expected Shannon and embedded Nushell versions;
- create bottles for fast installs once the formula builds correctly from the
  release source tarball;
- document the tap trust step required by modern Homebrew.

The formula should preserve Shannon's dependency model. Nushell and Reedline are
vendored source trees in this repo, so the source tarball used by Homebrew must
include them. The packaging design should explicitly verify that the tarball is
complete enough for a clean Homebrew source build without relying on local git
submodules, ignored vendor directories, or developer-only state.

## Requirements

The completed deployment path should provide:

- a `dist/shannon.rb` formula source of truth in this repo;
- a public Homebrew tap repository containing the published formula;
- a release source tarball asset with a sha256 recorded in the formula;
- a source-build path verified through Homebrew;
- a bottle path if supported on the release machine;
- install documentation in the README;
- an uninstall/upgrade story that follows standard Homebrew behavior;
- verification from a cold tap/install path, including the Homebrew 6.0+ trust
  gate if applicable.

The formula verification bar should include:

- `brew audit` / `brew style` or documented justification for third-party tap
  warnings;
- a local tap or public tap source-build verification;
- `brew test shannon`;
- `shannon --version` showing the Shannon version and embedded Nushell version;
- a simple shell execution smoke test through the installed binary;
- confirmation that no stale local checkout paths are embedded in the installed
  binary or package metadata.

## Open Questions

- How should publication authenticate to the `shannonshell` organization tap?
- Should the first release publish only a formula/source build, or also publish
  a bottle immediately?
- Should the formula install any shell integration files, completions, or
  Nushell configuration helpers, or only the `shannon` binary for the first
  release?
- What version should be published first through Homebrew? Experiment 1 used
  `0.5.6`, but that attempt failed against the wrong publication target.
  Experiment 2 uses `0.5.7` so the corrected `shannonshell` release path has an
  unambiguous artifact line.

## Experiments

- [Experiment 1: Formula, tap, and bottle deployment](01-formula-tap-and-bottle.md)
  — **Fail**
- [Experiment 2: Publish through shannonshell tap](02-publish-through-shannonshell-tap.md)
  — **Pass**

## Conclusion

Shannon is now installable through Homebrew from the organization-owned tap:

```bash
brew tap shannonshell/shannon
brew trust shannonshell/shannon
brew install shannon
```

The completed deployment publishes Shannon `0.5.7` from `shannonshell/shannon`,
with source release asset `shannon-0.5.7.tar.gz` and formula SHA
`9ee34faa76b8a60530f7360d172b1094f02a93e022a7d29decf635d90f9b995c`. The public
tap `shannonshell/homebrew-shannon` contains the formula and an `arm64_tahoe`
bottle with SHA
`55961cc18def8b261e7613785c6a150c95878a4fb852b3724af9b30c221eccf1`.

Verification covered `cargo test`, deterministic source tarball generation,
local Homebrew source-build proof, public source install, public bottle install,
`brew style`, `brew audit --new --strict`, `brew test`, runtime version and
execution smoke tests, and path-leak checks. Homebrew metadata confirmed the
installed formula came from `shannonshell/shannon` and the final install poured
from the published bottle.

The accidental `ryanxcharles/homebrew-shannon` tap repository, accidental
`v0.5.6` release, and accidental `v0.5.6` tag were removed. The local Homebrew
tap list contains `shannonshell/shannon`, not `ryanxcharles/shannon`.
