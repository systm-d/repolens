<!-- Auto-generated header - Do not edit manually -->
![Version](https://img.shields.io/badge/version-local-gray)

---

# Installation

Ce guide vous explique comment installer RepoLens sur votre système.

## Prérequis

- **Git** : Pour la gestion de version
- **GitHub CLI** (`gh`) : Optionnel, pour les fonctionnalités GitHub (installation via `gh auth login`)

## Docker (recommandé)

La méthode la plus simple pour utiliser RepoLens sans installation locale.

```bash
# Tirer l'image officielle
docker pull ghcr.io/systm-d/repolens:latest

# Auditer le répertoire courant
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens plan

# Générer un rapport
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens report --format json
```

Pour l'accès à l'API GitHub, montez votre configuration gh :

```bash
docker run --rm \
  -v "$(pwd)":/repo \
  -v ~/.config/gh:/home/repolens/.config/gh:ro \
  ghcr.io/systm-d/repolens plan
```

Tags disponibles :
- `latest` - Dernière version stable
- `v1.0.0`, `v1.1.0`, etc. - Versions spécifiques
- `sha-abc1234` - Commit spécifique

Voir [docs/docker.md](../docs/docker.md) pour plus de détails.

## Gestionnaires de paquets

### Homebrew (macOS/Linux)

```bash
brew tap systm-d/repolens
brew install repolens
```

Mise à jour :
```bash
brew upgrade repolens
```

### Scoop (Windows)

```powershell
scoop bucket add systm-d https://github.com/systm-d/scoop-bucket
scoop install repolens
```

Mise à jour :
```powershell
scoop update repolens
```

### AUR (Arch Linux)

```bash
# Avec yay
yay -S repolens

# Ou manuellement
git clone https://aur.archlinux.org/repolens.git
cd repolens
makepkg -si
```

### Debian/Ubuntu

```bash
# Ajouter le dépôt
curl -fsSL https://systm-d.github.io/repolens-apt/KEY.gpg | sudo gpg --dearmor -o /usr/share/keyrings/repolens.gpg
echo "deb [signed-by=/usr/share/keyrings/repolens.gpg] https://systm-d.github.io/repolens-apt stable main" | sudo tee /etc/apt/sources.list.d/repolens.list

# Installer
sudo apt update
sudo apt install repolens
```

## Installation via binaires pré-compilés

Des binaires pré-compilés sont disponibles pour toutes les plateformes majeures. Rendez-vous sur la [page Releases](https://github.com/systm-d/repolens/releases) pour télécharger la dernière version.

### Plateformes supportées

| Plateforme | Architecture | Archive |
|------------|-------------|---------|
| Linux | x86_64 | `repolens-linux-x86_64.tar.gz` |
| Linux | ARM64 | `repolens-linux-arm64.tar.gz` |
| macOS | Intel x86_64 | `repolens-darwin-x86_64.tar.gz` |
| macOS | Apple Silicon ARM64 | `repolens-darwin-arm64.tar.gz` |
| Windows | x86_64 | `repolens-windows-x86_64.zip` |

### Linux (x86_64)

```bash
curl -LO https://github.com/systm-d/repolens/releases/latest/download/repolens-linux-x86_64.tar.gz
tar xzf repolens-linux-x86_64.tar.gz
sudo mv repolens /usr/local/bin/
```

### Linux (ARM64)

```bash
curl -LO https://github.com/systm-d/repolens/releases/latest/download/repolens-linux-arm64.tar.gz
tar xzf repolens-linux-arm64.tar.gz
sudo mv repolens /usr/local/bin/
```

### macOS (Apple Silicon)

```bash
curl -LO https://github.com/systm-d/repolens/releases/latest/download/repolens-darwin-arm64.tar.gz
tar xzf repolens-darwin-arm64.tar.gz
sudo mv repolens /usr/local/bin/
```

### macOS (Intel)

```bash
curl -LO https://github.com/systm-d/repolens/releases/latest/download/repolens-darwin-x86_64.tar.gz
tar xzf repolens-darwin-x86_64.tar.gz
sudo mv repolens /usr/local/bin/
```

### Windows (x86_64)

```powershell
# Telecharger l'archive depuis la page Releases
Invoke-WebRequest -Uri https://github.com/systm-d/repolens/releases/latest/download/repolens-windows-x86_64.zip -OutFile repolens-windows-x86_64.zip
Expand-Archive repolens-windows-x86_64.zip -DestinationPath .
Move-Item repolens.exe C:\Users\$env:USERNAME\bin\
```

### Verifier les checksums

Chaque release inclut un fichier `checksums.sha256` pour verifier l'integrite des archives :

```bash
# Telecharger le fichier de checksums
curl -LO https://github.com/systm-d/repolens/releases/latest/download/checksums.sha256

# Verifier (Linux)
sha256sum -c checksums.sha256 --ignore-missing

# Verifier (macOS)
shasum -a 256 -c checksums.sha256 --ignore-missing
```

### Verifier l'installation

```bash
repolens --version
```

## Installation depuis les sources

> **Note** : L'installation depuis les sources necessite **Rust** version stable (1.70+ recommandee).

### 1. Cloner le repository

```bash
git clone https://github.com/systm-d/repolens.git
cd cli--repolens
```

### 2. Compiler le projet

```bash
# Compilation en mode release (recommandé)
cargo build --release

# Le binaire sera disponible à : target/release/repolens
```

### 3. Installer globalement (optionnel)

```bash
# Ajouter au PATH
sudo cp target/release/repolens /usr/local/bin/

# Ou créer un lien symbolique
sudo ln -s $(pwd)/target/release/repolens /usr/local/bin/repolens
```

### 4. Vérifier l'installation

```bash
repolens --help
```

## Installation de Rust

Si Rust n'est pas installé sur votre système :

```bash
# Installation via rustup (recommandé)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Suivre les instructions à l'écran
# Redémarrer le terminal après l'installation
```

Vérifier l'installation :

```bash
rustc --version
cargo --version
```

## Installation de GitHub CLI

Pour utiliser les fonctionnalités GitHub de RepoLens :

### Linux

```bash
# Via le gestionnaire de paquets
sudo apt install gh  # Debian/Ubuntu
sudo dnf install gh  # Fedora

# Ou via le script d'installation
curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | sudo tee /etc/apt/sources.list.d/github-cli.list > /dev/null
sudo apt update
sudo apt install gh
```

### macOS

```bash
brew install gh
```

### Authentification

```bash
gh auth login
```

## Installation via crates.io

RepoLens est publie sur [crates.io](https://crates.io/crates/repolens). C'est la methode d'installation la plus simple si vous avez deja Rust installe :

```bash
cargo install repolens
```

Pour mettre a jour vers la derniere version :

```bash
cargo install repolens --force
```

## Installation via binaires pre-compiles

Des binaires pre-compiles sont disponibles pour les plateformes suivantes sur la [page des releases](https://github.com/systm-d/repolens/releases) :

| Plateforme | Architecture | Fichier |
|---|---|---|
| Linux | x86_64 | `repolens-linux-x86_64.tar.gz` |
| Linux | ARM64 | `repolens-linux-aarch64.tar.gz` |
| macOS | Intel (x86_64) | `repolens-darwin-x86_64.tar.gz` |
| macOS | Apple Silicon (ARM64) | `repolens-darwin-aarch64.tar.gz` |
| Windows | x86_64 | `repolens-windows-x86_64.zip` |

### Linux / macOS

```bash
# Telecharger et extraire (exemple pour Linux x86_64)
curl -L https://github.com/systm-d/repolens/releases/latest/download/repolens-linux-x86_64.tar.gz | tar xz

# Rendre executable et deplacer dans le PATH
chmod +x repolens
sudo mv repolens /usr/local/bin/
```

### Windows

1. Telecharger `repolens-windows-x86_64.zip` depuis la [page des releases](https://github.com/systm-d/repolens/releases)
2. Extraire l'archive
3. Ajouter le dossier contenant `repolens.exe` a votre variable d'environnement `PATH`

## Utilisation via GitHub Action

RepoLens est disponible en tant qu'Action GitHub officielle pour integrer l'audit directement dans vos workflows CI/CD.

### Utilisation basique

```yaml
name: RepoLens Audit
on: [push, pull_request]

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: systm-d/repolens-action@v1
        with:
          preset: opensource
```

### Inputs disponibles

| Input | Description | Defaut |
|---|---|---|
| `preset` | Preset de configuration (`opensource`, `enterprise`, `strict`) | `opensource` |
| `format` | Format de sortie (`terminal`, `json`, `sarif`, `markdown`, `html`) | `terminal` |
| `output` | Chemin du fichier de sortie | - |
| `categories` | Categories a auditer (separees par des virgules) | toutes |
| `exclude` | Categories a exclure (separees par des virgules) | - |
| `verbose` | Niveau de verbosite (`0`-`3`) | `0` |
| `fail-on-error` | Echouer le workflow si des problemes sont detectes | `false` |

### Outputs disponibles

| Output | Description |
|---|---|
| `score` | Score global de l'audit |
| `report-path` | Chemin du rapport genere |
| `issues-count` | Nombre de problemes detectes |

### Exemple avance avec publication SARIF

```yaml
name: RepoLens Security Audit
on: [push]

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: systm-d/repolens-action@v1
        id: audit
        with:
          preset: strict
          format: sarif
          output: repolens-results.sarif
          fail-on-error: true
      - uses: github/codeql-action/upload-sarif@v3
        if: always()
        with:
          sarif_file: repolens-results.sarif
```

### Exemple audit multi-presets

```yaml
name: RepoLens Multi-Preset Audit
on: [pull_request]

jobs:
  audit:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        preset: [opensource, enterprise, strict]
    steps:
      - uses: actions/checkout@v4
      - uses: systm-d/repolens-action@v1
        with:
          preset: ${{ matrix.preset }}
          format: markdown
          output: report-${{ matrix.preset }}.md
```

## Dépannage

### Erreur de compilation

```bash
# Nettoyer et reconstruire
cargo clean
cargo build --release
```

### Problèmes avec les dépendances

```bash
# Mettre à jour les dépendances
cargo update

# Vérifier les versions
cargo tree
```

### Erreur "command not found"

Assurez-vous que le binaire est dans votre PATH :

```bash
# Vérifier le PATH
echo $PATH

# Ajouter manuellement si nécessaire
export PATH="$PATH:$(pwd)/target/release"
```

## Prochaines étapes

Une fois installé, consultez le [Guide d'utilisation](Guide-d-utilisation) pour commencer à utiliser RepoLens.
