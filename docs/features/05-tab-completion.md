# Tab Completion

Press **Tab** to complete file and directory names.

## Basic Usage

Type part of a filename and press Tab:

```
[bash] ~/project > cat Car<Tab>
```

If there's one match, it's inserted. If there are multiple matches, a menu
appears:

```
Cargo.lock  Cargo.toml
```

Press Tab again to cycle through options. Press Enter to accept, or keep typing
to narrow the results.

## Directory Completion

Directories are completed with a trailing `/` and no space, so you can keep
tabbing into subdirectories:

```
[bash] ~/project > ls sr<Tab>
[bash] ~/project > ls src/<Tab>
completer.rs  executor.rs  highlighter.rs  lib.rs  main.rs  prompt.rs  shell.rs
```

Files get a trailing space instead, since you're likely done with that argument.

## Hidden Files

Hidden files (starting with `.`) are excluded by default. To see them, type a
`.` before pressing Tab:

```
[bash] ~/project > ls .<Tab>
.git/  .gitignore
```

## Tilde Expansion

Tab completion understands `~` as your home directory:

```
[bash] ~/project > cat ~/Doc<Tab>
[bash] ~/project > cat ~/Documents/
```

The `~` is preserved in the completion — it won't expand to an absolute path.

## Sort Order

Completions are sorted with directories first, then files. Within each group,
entries are sorted alphabetically.
