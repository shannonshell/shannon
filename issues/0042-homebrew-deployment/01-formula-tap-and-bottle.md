# Experiment 1: Formula, Tap, and Bottle Deployment

## Description

Create and publish Shannon's first Homebrew formula deployment path.

This experiment intentionally covers the whole deployment path rather than only
creating a local formula. A local-only formula would not satisfy Issue 42's
requirement for a public tap and cold install path. The implementation should
still have a local preflight before any irreversible publication step, so a bad
formula, incomplete source tarball, or broken install test fails before pushing
public artifacts.

The immediate tap target is `shannonshell/homebrew-shannon`, installed as:

```bash
brew tap shannonshell/shannon
brew trust shannonshell/shannon
brew install shannon
```

This tap owner is mandatory. Shannon has a GitHub organization, `shannonshell`,
and the Homebrew tap must live there. Do not publish a Shannon tap as
`ryanxcharles/homebrew-shannon`.

The first Homebrew release version is `0.5.6`. The original design-review fix
used `0.5.5`, matching the package version at the time, but publication
preflight found that tag already exists on `origin` and points at pre-Homebrew,
pre-Nushell-0.113.1 code. This experiment must therefore bump Shannon's package
version to `0.5.6` and publish that new tag rather than overwrite `v0.5.5`.

## Changes

Planned repo changes:

- `dist/shannon.rb` — add the Homebrew formula source of truth.
- `scripts/make-source-tarball.sh` — add a release-asset tarball builder that
  archives the full tracked source tree with `nushell/` and `reedline/`.
- `Cargo.toml`, `Cargo.lock`, `nushell/Cargo.toml`,
  `nushell/crates/nu-cli/Cargo.toml`, `nushell/crates/nu-lsp/Cargo.toml`, and
  lockfiles — bump Shannon-owned crate versions from `0.5.5` to `0.5.6`.
- `README.md` — lead installation with the Homebrew tap path and keep Cargo
  install/build instructions as source/developer alternatives.
- `issues/0042-homebrew-deployment/README.md` — update Experiment 1 status when
  the result is known.
- `issues/0042-homebrew-deployment/01-formula-tap-and-bottle.md` — record design
  review, result, completion review, and conclusion.

Planned external publication changes:

- `shannonshell/shannon` — create or update GitHub Release `v0.5.6` with a
  source tarball asset named `shannon-0.5.6.tar.gz`.
- `shannonshell/homebrew-shannon` — create or update a public Homebrew tap with
  `Formula/shannon.rb`, a README containing the tap/trust/install commands, and
  a bottle release if bottling succeeds.

Formula shape:

- `class Shannon < Formula`.
- `desc` and `homepage` from the root package metadata.
- `url` points at the uploaded GitHub Release source tarball asset, not GitHub's
  generated `/archive/` tarballs.
- `sha256` pins that uploaded source tarball.
- `license "MIT"`.
- `depends_on "rust" => :build`.
- `def install` uses Homebrew's standard Rust pattern:
  `system "cargo", "install", *std_cargo_args`.
- `test do` checks:
  - `shannon --version` includes `0.5.6` and `nushell 0.113.1`;
  - `shannon -c '1 + 2'` prints `3`.

Publication sequence:

1. Preflight local state:
   - confirm the Shannon repo is clean;
   - confirm GitHub CLI auth has permission to publish to `shannonshell`;
   - confirm whether `v0.5.6` already exists on `shannonshell/shannon`;
   - confirm whether `shannonshell/homebrew-shannon` already exists.
2. Record design review approval and commit this experiment plan before
   implementation.
3. Implement `dist/shannon.rb`, `scripts/make-source-tarball.sh`, and README
   documentation.
4. Build a local source tarball from the current implementation commit with:
   `scripts/make-source-tarball.sh HEAD`.
5. Create a temporary local tap formula pointing at the `file://` tarball and
   its sha256.
6. Verify the local formula before publishing:
   - `brew style`;
   - `brew install --build-from-source` from the local tap formula;
   - `brew test shannon`;
   - `$(brew --prefix)/bin/shannon --version`;
   - `$(brew --prefix)/bin/shannon -c '1 + 2'`;
   - confirm the installed binary does not reference this checkout path in
     obvious string output such as
     `strings $(brew --prefix)/bin/shannon | rg "$PWD"`.
7. If local verification passes, publish:
   - push the implementation commit to `origin`;
   - create tag `v0.5.6` pointing at that commit;
   - create the GitHub Release with `shannon-0.5.6.tar.gz`;
   - update the tap formula URL and sha256 to the uploaded release asset;
   - create/push `shannonshell/homebrew-shannon` if it does not exist.
8. Run public formula checks after the release URL exists:
   - `brew style shannonshell/shannon/shannon`;
   - `brew audit --new --strict shannonshell/shannon/shannon`;
   - record and justify any expected third-party tap warnings.
9. Build a bottle if Homebrew can bottle the formula on this machine:
   - `brew install --build-bottle shannonshell/shannon/shannon`;
   - `brew bottle --json --no-rebuild --root-url=...`;
   - create a tap release and upload the bottle;
   - merge the emitted bottle block into `Formula/shannon.rb`;
   - push the tap.
10. Verify the public cold install:
    - uninstall any local `shannon` formula;
    - untap/re-tap `shannonshell/shannon`;
    - prove untrusted install refusal if Homebrew requires trust;
    - `brew trust shannonshell/shannon`;
    - `brew install shannon`;
    - verify version, nu command execution, `brew test shannon`, and bottle
      pour/source-build behavior.

11. Record the result, run completion review, fix real findings, and commit the
    result separately from this plan.

## Design Review

Initial Codex design review: **Changes required**.

Required findings:

- The first Homebrew release version was unresolved even though the plan used
  `${VERSION}` for tags and release assets.
- The local preflight required `brew audit --new --strict` against a temporary
  `file://` formula before a public URL existed, which would likely produce
  non-actionable online audit failures.

Optional finding:

- The formula should use Homebrew's standard Rust install pattern instead of
  hand-running `cargo build` and manually installing `target/release/shannon`.

Fixes applied before re-review:

- Pinned Experiment 1 to Shannon version `0.5.6`.
- Split verification into local pre-publication proof without online audit, then
  public `brew style` / `brew audit --new --strict` after the release URL and
  tap formula exist.
- Changed the formula design to use
  `system "cargo", "install", *std_cargo_args`.

Second Codex design review: **Approved**. No required findings remained. The
reviewer noted one formatting nit in the nested cold-install checklist; that nit
was fixed before the plan commit.

Publication preflight after local formula proof found that `v0.5.5` already
exists on `origin` at commit `1049bff25`, before Issue 41's Nushell 0.113.1
upgrade. Reusing `0.5.5` would require overwriting an existing tag, which this
plan forbids. The plan is therefore amended to publish `0.5.6` and bump
Shannon-owned package versions before the public release.

Codex review of the `0.5.6` plan amendment: **Approved**. No required findings.
The reviewer noted one optional implementation reminder: update
`dist/shannon.rb` from the initial local-proof `0.5.5` values to `0.5.6` before
publication.

## Verification

Pass criteria:

- `dist/shannon.rb` is present and matches the published tap formula, except for
  any bottle block owned by the tap.
- `scripts/make-source-tarball.sh` creates a source tarball containing all
  tracked files required for a clean Homebrew build, including `nushell/` and
  `reedline/`.
- The formula builds Shannon through Homebrew from the source tarball.
- `brew test shannon` passes.
- The installed `shannon --version` reports the Shannon version and embedded
  Nushell `0.113.1`.
- The installed binary can run a simple Nushell command with `shannon -c`.
- A public tap install works from `shannonshell/homebrew-shannon`.
- The README documents Homebrew installation and source/developer alternatives.
- If a bottle is published, the cold install pours it; if bottling is blocked,
  the issue records why and verifies the source-build path instead.
- Plan and result commits are separate, and design/result reviews are recorded.

Fail criteria:

- The formula cannot build from a release tarball without local checkout state.
- The source tarball omits vendored `nushell/` or `reedline/` files required by
  the build.
- The formula requires undocumented environment variables or manual pre-setup.
- Homebrew install produces a `shannon` binary that cannot start or run a simple
  Nushell command.
- Publishing would overwrite an existing GitHub release or tap artifact without
  explicit approval and a recovery plan.

## Result

**Result:** Fail

The experiment was stopped because the implementation and publication path
targeted the wrong Homebrew tap owner. Shannon has an organization,
`shannonshell`, and the tap must be `shannonshell/homebrew-shannon`, installed
as `brew tap shannonshell/shannon`.

The original approved plan also targeted the wrong owner. After the failure, the
issue README and this experiment file were corrected to show the required
organization tap, and the local README/formula artifacts were updated to use
`shannonshell` URLs instead of `ryanxcharles` URLs.

The failed path created unintended external artifacts under the personal
account:

- `ryanxcharles/homebrew-shannon` was created and contains the local tap
  commits.
- `ryanxcharles/shannon` received pushed Issue 42 commits, tag `v0.5.6`, and a
  GitHub Release `v0.5.6` with `shannon-0.5.6.tar.gz`.

Do not continue from those artifacts. The next experiment should either remove
or explicitly supersede the accidental personal tap/release artifacts, then
publish through `shannonshell/homebrew-shannon`.

Because `v0.5.6` now exists on the personal fork and points at the failed
publication path, the next experiment should use a fresh patch version for the
organization-owned release unless an explicit cleanup plan removes the personal
tag/release and safely reclaims the version.

## Conclusion

The formula shape and local Homebrew source-build proof were useful, but the
publication target was wrong. The correct next step is a new experiment focused
on the `shannonshell` organization tap and cleanup/supersession of the
accidental personal-account artifacts.
