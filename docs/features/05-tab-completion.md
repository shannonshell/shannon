# Tab Completion

Press **Tab** to complete file names, directory names, commands, subcommands,
and flags.

## Command-Aware Completions

Shannon knows the subcommands and flags for nearly 1,000 commands. Type a
command, press space, then Tab:

```
[bash] ~/project > git <Tab>
add  bisect  branch  checkout  clone  commit  diff  fetch  log  merge  pull  push  rebase  reset  status  ...
```

Type part of a subcommand to narrow results:

```
[bash] ~/project > git com<Tab>
commit
```

### Flag Completion

After a subcommand, type `--` and press Tab to see available flags:

```
[bash] ~/project > git commit --<Tab>
--message  --amend  --all  --verbose  --dry-run  ...
```

### Where Completions Come From

Shannon's command completions are parsed from
[fish shell's](https://fishshell.com/) community-maintained completion files
at build time. This covers 983 commands including git, docker, cargo, npm,
ssh, curl, and hundreds more. The completions are baked into the binary — no
runtime dependency on fish.

Completions work in all shell modes (bash, nushell, fish).

## File and Directory Completion

When no command-specific completions match, Tab falls back to file and
directory completion:

```
[bash] ~/project > cat Car<Tab>
Cargo.lock  Cargo.toml
```

### Directory Completion

Directories are completed with a trailing `/` and no space, so you can keep
tabbing into subdirectories:

```
[bash] ~/project > ls sr<Tab>
[bash] ~/project > ls src/<Tab>
completer.rs  executor.rs  highlighter.rs  lib.rs  main.rs  prompt.rs  shell.rs
```

Files get a trailing space instead, since you're likely done with that argument.

### Hidden Files

Hidden files (starting with `.`) are excluded by default. To see them, type a
`.` before pressing Tab:

```
[bash] ~/project > ls .<Tab>
.git/  .gitignore
```

### Tilde Expansion

Tab completion understands `~` as your home directory:

```
[bash] ~/project > cat ~/Doc<Tab>
[bash] ~/project > cat ~/Documents/
```

The `~` is preserved in the completion — it won't expand to an absolute path.

## Sort Order

Completions are sorted with directories first, then files. Within each group,
entries are sorted alphabetically. Command completions are sorted
alphabetically.
