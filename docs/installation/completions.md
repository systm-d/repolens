# Shell completions

RepoLens ships a hidden `completions` subcommand that prints a completion
script for the requested shell on standard output. Pipe the output into
the location your shell loads completions from.

Supported shells: `bash`, `zsh`, `fish`, `powershell`, `elvish`, `nushell`.

```bash
repolens completions <shell>
```

## Bash

Append to `~/.bashrc` (or any file sourced by it):

```bash
eval "$(repolens completions bash)"
```

For a system-wide install on Linux:

```bash
repolens completions bash | sudo tee /etc/bash_completion.d/repolens > /dev/null
```

## Zsh

The recommended approach is to drop the script in a directory listed in
`$fpath`, then run `compinit`:

```zsh
mkdir -p ~/.zsh/completions
repolens completions zsh > ~/.zsh/completions/_repolens
```

Add the following to `~/.zshrc` (before `compinit`):

```zsh
fpath=(~/.zsh/completions $fpath)
autoload -Uz compinit && compinit
```

Alternatively, source the script directly:

```zsh
eval "$(repolens completions zsh)"
```

## Fish

Fish auto-loads completions from `~/.config/fish/completions/`:

```fish
mkdir -p ~/.config/fish/completions
repolens completions fish > ~/.config/fish/completions/repolens.fish
```

## PowerShell

Append to your PowerShell profile (`$PROFILE`):

```powershell
repolens completions powershell | Out-String | Invoke-Expression
```

## Elvish

```elvish
mkdir -p ~/.config/elvish/lib
repolens completions elvish > ~/.config/elvish/lib/repolens.elv
echo 'use repolens' >> ~/.config/elvish/rc.elv
```

## Nushell

```nushell
mkdir ~/.config/nushell/completions
repolens completions nushell | save -f ~/.config/nushell/completions/repolens.nu
# Then `source` the file from your `config.nu`
```

## What gets completed

- Subcommand names (`init`, `plan`, `apply`, `report`, `compare`, ...).
- Global flags `-c/--config` (file paths) and `-C/--directory` (directory paths).
- `--only` and `--skip` cycle through the 11 valid rule categories:
  `secrets`, `files`, `docs`, `security`, `workflows`, `quality`,
  `dependencies`, `licenses`, `docker`, `git`, `custom`.
- `--preset` (`opensource`, `enterprise`, `strict`) and per-command
  `--format` enums are derived automatically from the CLI definition.

## Regenerating after upgrades

The script is tied to the CLI surface of the version that produced it.
Re-run `repolens completions <shell> > <path>` after upgrading RepoLens
so the completions stay in sync with new flags and subcommands.
