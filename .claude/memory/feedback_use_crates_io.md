---
name: feedback_use_crates_io
description: Use crates.io dependencies, not vendored path dependencies, for Rust packages
type: feedback
---

Use normal crates.io dependencies for Rust packages, not path dependencies to vendored copies.

**Why:** The vendor directory is for reference/research, not for building against. User explicitly corrected this.

**How to apply:** In Cargo.toml, use version strings (e.g. `reedline = "0.46.0"`) not path deps (e.g. `reedline = { path = "vendor/reedline" }`).
