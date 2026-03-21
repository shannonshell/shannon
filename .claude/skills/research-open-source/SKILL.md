---
name: research-open-source
description: "Use the local copies of each repo when doing research. Use this when doing research on any open source repo."
---

# Research Open Source

When researching open source code, use the local clones in `vendor/` instead of
searching the internet. The full source code is already on disk — there is no
reason to fetch it remotely.

## Check what's available

Before starting any research, check what repos are already cloned:

```bash
ls ~/dev/olshell/vendor/
```

### Currently available repos

| Repo | Path |
|------|------|
| reedline | `vendor/reedline/` |
| nushell | `vendor/nushell/` |
| bash | `vendor/bash/` |
| zsh | `vendor/zsh/` |
| fish | `vendor/fish/` |
| PowerShell | `vendor/powershell/` |
| elvish | `vendor/elvish/` |

## Research workflow

1. **Identify which repo has the code you need.** Check the vendor directory.
2. **Read the source directly.** Use Grep, Glob, and Read tools on the local
   clone. No web fetching needed.
3. **If the repo is not cloned yet**, ask the user before cloning it. Do not
   clone repos without confirmation.

## Cloning a new repo

If research requires a repo that is not yet in `vendor/`:

1. **Ask the user** if it is OK to clone the repo into `vendor/`.
2. Clone it: `git clone <url> vendor/<name>/`
3. The `vendor/.gitignore` already ignores all contents.

Never clone without user approval.
