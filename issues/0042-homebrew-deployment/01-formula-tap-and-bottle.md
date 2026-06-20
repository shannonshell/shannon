# Experiment 1: Formula, Tap, and Bottle Deployment

## Description

Create and publish Shannon's first Homebrew formula deployment path.

This experiment intentionally covers the whole deployment path rather than only
creating a local formula. A local-only formula would not satisfy Issue 42's
requirement for a public tap and cold install path. The implementation should
still have a local preflight before any irreversible publication step, so a bad
formula, incomplete source tarball, or broken install test fails before pushing
public artifacts.

The immediate tap target is `ryanxcharles/homebrew-shannon`, installed as:

```bash
brew tap ryanxcharles/shannon
brew trust ryanxcharles/shannon
brew install shannon
```

This choice is deliberate for the first publication because this checkout's
`origin` is `ryanxcharles/shannon` and local GitHub authentication is for
`ryanxcharles`. The issue can later add a small follow-up or mirror step for
`shannonshell/homebrew-shannon` if that organization should own the canonical
tap.

The first Homebrew release version is `0.5.5`, matching the current root
`Cargo.toml` package version. Any different version must be recorded as a new
plan change before implementation or publication begins.

## Changes

Planned repo changes:

- `dist/shannon.rb` — add the Homebrew formula source of truth.
- `scripts/make-source-tarball.sh` — add a release-asset tarball builder that
  archives the full tracked source tree with `nushell/` and `reedline/`.
- `README.md` — lead installation with the Homebrew tap path and keep Cargo
  install/build instructions as source/developer alternatives.
- `issues/0042-homebrew-deployment/README.md` — update Experiment 1 status when
  the result is known.
- `issues/0042-homebrew-deployment/01-formula-tap-and-bottle.md` — record design
  review, result, completion review, and conclusion.

Planned external publication changes:

- `ryanxcharles/shannon` — create or update GitHub Release `v0.5.5` with a
  source tarball asset named `shannon-0.5.5.tar.gz`.
- `ryanxcharles/homebrew-shannon` — create or update a public Homebrew tap with
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
  - `shannon --version` includes the formula version and `nushell 0.113.1`;
  - `shannon -c '1 + 2'` prints `3`.

Publication sequence:

1. Preflight local state:
   - confirm the Shannon repo is clean;
   - confirm `origin` is `ryanxcharles/shannon`;
   - confirm GitHub CLI auth is for `ryanxcharles`;
   - confirm whether `v0.5.5` already exists on `ryanxcharles/shannon`;
   - confirm whether `ryanxcharles/homebrew-shannon` already exists.
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
   - create tag `v0.5.5` pointing at that commit;
   - create the GitHub Release with `shannon-0.5.5.tar.gz`;
   - update the tap formula URL and sha256 to the uploaded release asset;
   - create/push `ryanxcharles/homebrew-shannon` if it does not exist.
8. Run public formula checks after the release URL exists:
   - `brew style ryanxcharles/shannon/shannon`;
   - `brew audit --new --strict ryanxcharles/shannon/shannon`;
   - record and justify any expected third-party tap warnings.
9. Build a bottle if Homebrew can bottle the formula on this machine:
   - `brew install --build-bottle ryanxcharles/shannon/shannon`;
   - `brew bottle --json --no-rebuild --root-url=...`;
   - create a tap release and upload the bottle;
   - merge the emitted bottle block into `Formula/shannon.rb`;
   - push the tap.
10. Verify the public cold install:
    - uninstall any local `shannon` formula;
    - untap/re-tap `ryanxcharles/shannon`;
    - prove untrusted install refusal if Homebrew requires trust;
    - `brew trust ryanxcharles/shannon`;
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

- Pinned Experiment 1 to Shannon version `0.5.5`.
- Split verification into local pre-publication proof without online audit, then
  public `brew style` / `brew audit --new --strict` after the release URL and
  tap formula exist.
- Changed the formula design to use
  `system "cargo", "install", *std_cargo_args`.

Second Codex design review: **Approved**. No required findings remained. The
reviewer noted one formatting nit in the nested cold-install checklist; that nit
was fixed before the plan commit.

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
- A public tap install works from `ryanxcharles/homebrew-shannon`.
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
