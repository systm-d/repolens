# Release Process

Ce document décrit le processus de release pour RepoLens.

## Vue d'ensemble

Le projet utilise deux types de builds :

1. **Nightly Builds** : Construits automatiquement à chaque push sur `main`/`master`
2. **Release Builds** : Construits lorsqu'un tag de version est créé (format: `v*.*.*`)

## Nightly Builds

Les nightly builds sont automatiquement créés à chaque push sur la branche principale.

### Caractéristiques

- **Version** : `{base_version}-nightly.{date}-{commit_sha}`
  - Exemple : `1.0.0-nightly.20260124-143022-a1b2c3d`
- **Tag** : `nightly-{date}-{commit_sha}`
- **Prerelease** : Oui (marqué comme pre-release sur GitHub)
- **Rétention** : 30 jours

### Utilisation

Les nightly builds sont disponibles dans les [Releases GitHub](https://github.com/systm-d/repolens/releases) avec le préfixe "Nightly Build".

⚠️ **Attention** : Les nightly builds peuvent être instables. Utilisez-les à vos risques et périls.

## Releases

Les releases sont créées automatiquement depuis la CI avec auto-incrémentation de version.

### Créer une Release depuis la CI

#### Méthode recommandée : Workflow GitHub Actions

1. Allez sur la page [Actions](https://github.com/systm-d/repolens/actions) de votre dépôt
2. Sélectionnez le workflow **"Create Release"**
3. Cliquez sur **"Run workflow"**
4. Choisissez le type d'incrémentation :
   - **patch** : Corrections de bugs (1.0.0 → 1.0.1)
   - **minor** : Nouvelles fonctionnalités (1.0.0 → 1.1.0)
   - **major** : Changements incompatibles (1.0.0 → 2.0.0)
5. Cochez **"Create and push tag automatically"** (recommandé)
6. Cliquez sur **"Run workflow"**

Le workflow va automatiquement :
1. Calculer la prochaine version en fonction du dernier tag
2. Mettre à jour `Cargo.toml` avec la nouvelle version
3. Exécuter les tests
4. Vérifier le formatage et clippy
5. Créer un commit avec la nouvelle version
6. Créer et pousser le tag
7. Déclencher automatiquement le workflow de build et release

### Créer une Release manuellement

#### Méthode 1 : Script automatique

```bash
./scripts/release.sh 1.0.0
```

Le script va :
1. Valider le format de version
2. Vérifier que le working directory est propre
3. Mettre à jour `Cargo.toml`
4. Exécuter les tests
5. Vérifier le formatage et clippy
6. Créer un commit
7. Créer un tag annoté
8. Afficher les instructions pour pousser

Ensuite, poussez le commit et le tag :

```bash
git push origin main
git push origin v1.0.0
```

#### Méthode 2 : Manuel

1. Mettre à jour la version dans `Cargo.toml` :
   ```toml
   version = "1.0.0"
   ```

2. Créer un commit :
   ```bash
   git add Cargo.toml
   git commit -m "chore: bump version to 1.0.0"
   ```

3. Créer un tag annoté :
   ```bash
   git tag -a v1.0.0 -m "Release 1.0.0"
   ```

4. Pousser le commit et le tag :
   ```bash
   git push origin main
   git push origin v1.0.0
   ```

### Format de version

Les versions doivent suivre [Semantic Versioning](https://semver.org/) :

- Format : `MAJOR.MINOR.PATCH`
- Exemples : `1.0.0`, `1.1.0`, `2.0.0`, `2.1.3`
- Pré-releases : `1.0.0-alpha.1`, `1.0.0-beta.2`

### Auto-incrémentation de version

Le système calcule automatiquement la prochaine version en fonction :
- Du dernier tag Git (ou de la version dans `Cargo.toml` si aucun tag n'existe)
- Du type d'incrémentation choisi (patch, minor, major)

**Exemples :**
- Dernière version : `1.0.0`
  - **patch** → `1.0.1`
  - **minor** → `1.1.0`
  - **major** → `2.0.0`

### Processus automatique

Lorsqu'un tag est poussé (automatiquement ou manuellement), GitHub Actions va automatiquement :

1. **Build** :
   - Compiler le binaire en mode release
   - Optimiser avec LTO et strip
   - Créer des checksums (SHA256, SHA512, MD5)
   - Créer des archives (tar.gz, zip)

2. **Changelog** :
   - Générer automatiquement le CHANGELOG à partir des commits
   - Mettre à jour `CHANGELOG.md` dans le dépôt

3. **Release** :
   - Créer une release GitHub avec tous les artefacts
   - Inclure les instructions d'installation
   - Inclure les checksums pour vérification

### Artefacts de Release

Chaque release contient :

- `repolens` : Binaire exécutable
- `repolens.sha256` : Checksum SHA256
- `repolens.sha512` : Checksum SHA512
- `repolens.md5` : Checksum MD5
- `repolens-linux-x86_64.tar.gz` : Archive tar.gz
- `repolens-linux-x86_64.zip` : Archive zip

### Vérification des artefacts

```bash
# Télécharger les fichiers
wget https://github.com/systm-d/repolens/releases/download/v1.0.0/repolens
wget https://github.com/systm-d/repolens/releases/download/v1.0.0/repolens.sha256

# Vérifier
sha256sum -c repolens.sha256
```

### Installation

```bash
# Depuis tar.gz
tar xzf repolens-linux-x86_64.tar.gz
sudo mv repolens /usr/local/bin/

# Depuis zip
unzip repolens-linux-x86_64.zip
sudo mv repolens /usr/local/bin/

# Vérifier l'installation
repolens --version
```

## CHANGELOG

Le CHANGELOG est généré automatiquement lors des releases. Il suit le format [Keep a Changelog](https://keepachangelog.com/).

### Format des commits

Pour une meilleure génération automatique du CHANGELOG, utilisez des [Conventional Commits](https://www.conventionalcommits.org/) :

- `feat:` : Nouvelles fonctionnalités
- `fix:` : Corrections de bugs
- `perf:` : Améliorations de performance
- `refactor:` : Refactorisation
- `chore:` : Tâches de maintenance
- `breaking:` : Changements incompatibles
- `security:` : Corrections de sécurité

Exemples :

```
feat: add support for custom rule configurations
fix: correct gitignore update logic
perf: optimize file scanning performance
breaking: change default preset behavior
security: fix secret detection false positives
```

### Génération manuelle du CHANGELOG

```bash
./scripts/generate-changelog.sh [from_tag] [to_tag]
```

Exemple :

```bash
./scripts/generate-changelog.sh v1.0.0 v1.0.0
```

## Workflow GitHub Actions

### Nightly Build (`nightly.yml`)

- **Déclencheur** : Push sur `main`/`master` (sans tag)
- **Actions** :
  - Build avec version nightly
  - Création d'artefacts
  - Création d'une release pre-release

### Release (`release.yml`)

- **Déclencheur** : Push d'un tag `v*.*.*`
- **Actions** :
  - Build avec version du tag
  - Génération du CHANGELOG
  - Création d'une release complète
  - Mise à jour du CHANGELOG.md

## Bonnes pratiques

1. **Versioning** :
   - Utilisez Semantic Versioning
   - Incrémentez MAJOR pour les breaking changes
   - Incrémentez MINOR pour les nouvelles fonctionnalités
   - Incrémentez PATCH pour les corrections de bugs

2. **Commits** :
   - Utilisez des messages de commit clairs
   - Préférez les Conventional Commits
   - Un commit = une modification logique

3. **Tests** :
   - Assurez-vous que tous les tests passent avant de créer une release
   - Vérifiez que le code est formaté (`cargo fmt`)
   - Vérifiez que clippy ne trouve pas d'erreurs

4. **Documentation** :
   - Mettez à jour le README si nécessaire
   - Documentez les breaking changes
   - Mettez à jour les exemples

## Dépannage

### Le workflow ne se déclenche pas

- Vérifiez que le tag suit le format `v*.*.*`
- Vérifiez que le tag a été poussé : `git push origin v1.0.0`

### Le CHANGELOG est vide

- Vérifiez qu'il y a des commits entre les tags
- Utilisez le script manuel pour générer le CHANGELOG

### Les artefacts ne sont pas créés

- Vérifiez les logs GitHub Actions
- Vérifiez que le build a réussi
- Vérifiez les permissions du workflow
