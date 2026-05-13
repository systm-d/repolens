<!-- Auto-generated header - Do not edit manually -->
![Version](https://img.shields.io/badge/version-local-gray)

---

# Changelog Automatique

RepoLens génère automatiquement le CHANGELOG à partir des commits Git en suivant le format [Keep a Changelog](https://keepachangelog.com/).

## Fonctionnement

Le changelog est généré automatiquement lors des releases via le workflow GitHub Actions `.github/workflows/release.yml`. Il utilise le script `scripts/generate-changelog.sh` pour analyser les commits entre deux tags.

## Format des commits

Pour une meilleure génération automatique du CHANGELOG, utilisez des [Conventional Commits](https://www.conventionalcommits.org/) :

### Types de commits supportés

- `feat` ou `feature` : Nouvelles fonctionnalités → Section **Added**
- `fix` ou `bugfix` : Corrections de bugs → Section **Fixed**
- `perf` : Améliorations de performance → Section **Changed**
- `refactor` : Refactorisation → Section **Changed**
- `chore` : Tâches de maintenance → Section **Changed**
- `docs` : Documentation → Section **Changed**
- `style` : Formatage → Section **Changed**
- `test` : Tests → Section **Changed**
- `build` : Build → Section **Changed**
- `ci` : CI/CD → Section **Changed**
- `security` : Sécurité → Section **Security**

### Exemples de commits

```bash
# Nouvelle fonctionnalité
git commit -m "feat: Ajout de la vérification des dépendances"

# Correction de bug
git commit -m "fix: Correction de la détection des secrets"

# Changement avec breaking change
git commit -m "feat!: Modification de l'API de configuration"

# Sécurité
git commit -m "security: Correction de la vulnérabilité XSS"
```

### Breaking Changes

Pour indiquer un changement cassant (breaking change), utilisez `!` après le type :

```bash
git commit -m "feat!: Modification de l'API de configuration"
```

Ou utilisez le footer `BREAKING CHANGE` dans le corps du commit :

```bash
git commit -m "feat: Nouvelle API

BREAKING CHANGE: L'ancienne API n'est plus supportée"
```

## Génération manuelle

Vous pouvez générer le changelog manuellement avec le script :

```bash
# Générer le changelog entre deux tags
./scripts/generate-changelog.sh v1.0.0 v1.1.0

# Générer le changelog depuis le dernier tag jusqu'à HEAD
./scripts/generate-changelog.sh v1.0.0 HEAD

# Générer le changelog depuis le début du projet
./scripts/generate-changelog.sh $(git rev-list --max-parents=0 HEAD) HEAD
```

## Format du CHANGELOG

Le changelog généré suit le format [Keep a Changelog](https://keepachangelog.com/) :

```markdown
## [1.1.0] - 2026-01-28

### BREAKING CHANGES
- Modification de l'API de configuration (#123)

### Added
- Ajout de la vérification des dépendances (#124)
- Support des règles personnalisées (#125)

### Fixed
- Correction de la détection des secrets (#126)

### Changed
- Amélioration de la performance (#127)

### Security
- Correction de la vulnérabilité XSS (#128)

[1.1.0]: https://github.com/systm-d/repolens/releases/tag/v1.1.0
```

## Intégration dans les workflows

### Workflow de release

Le workflow `.github/workflows/release.yml` génère automatiquement le changelog lors des releases :

1. **Génération** : Le job `changelog` génère une nouvelle entrée
2. **Intégration** : L'entrée est ajoutée au début du `CHANGELOG.md`
3. **Commit** : Le fichier est automatiquement commité dans le dépôt
4. **Release** : Le changelog est inclus dans les notes de release GitHub

### Workflow de synchronisation wiki

Le workflow `.github/workflows/sync-wiki.yml` peut synchroniser le changelog vers le wiki si configuré.

## Configuration

### Personnalisation du script

Le script `scripts/generate-changelog.sh` peut être personnalisé pour :

- Ajouter des catégories personnalisées
- Modifier le format de sortie
- Filtrer certains types de commits
- Ajouter des métadonnées supplémentaires

### Exclure des commits

Pour exclure certains commits du changelog, utilisez des conventions de nommage :

```bash
# Les commits commençant par "chore:" ou "docs:" peuvent être ignorés
# selon votre configuration
```

## Bonnes pratiques

1. **Utilisez des Conventional Commits** : Facilite la génération automatique
2. **Messages de commit clairs** : Aidez les utilisateurs à comprendre les changements
3. **Groupez les changements** : Plusieurs petits changements peuvent être regroupés
4. **Documentez les breaking changes** : Utilisez `!` ou le footer `BREAKING CHANGE`
5. **Vérifiez avant de release** : Relisez le changelog généré avant la release

## Exemples

### Release avec nouvelles fonctionnalités

```bash
# Commits
feat: Ajout de la vérification des dépendances (#124)
feat: Support des règles personnalisées (#125)
fix: Correction de la détection des secrets (#126)

# CHANGELOG généré
## [1.1.0] - 2026-01-28

### Added
- Ajout de la vérification des dépendances (#124)
- Support des règles personnalisées (#125)

### Fixed
- Correction de la détection des secrets (#126)
```

### Release avec breaking change

```bash
# Commit
feat!: Modification de l'API de configuration (#123)

# CHANGELOG généré
## [1.1.0] - 2026-01-28

### BREAKING CHANGES
- Modification de l'API de configuration (#123)
```

## Dépannage

### Le changelog est vide

Si le changelog généré est vide :

1. Vérifiez que les commits suivent le format Conventional Commits
2. Vérifiez que les tags existent : `git tag -l`
3. Vérifiez la plage de commits : `git log v1.0.0..v1.1.0`

### Format incorrect

Si le format n'est pas correct :

1. Vérifiez que vous utilisez des Conventional Commits
2. Vérifiez la syntaxe du script `scripts/generate-changelog.sh`
3. Testez manuellement avec le script

### Commits manquants

Si certains commits n'apparaissent pas :

1. Vérifiez qu'ils sont dans la plage de tags spécifiée
2. Vérifiez qu'ils suivent le format Conventional Commits
3. Vérifiez qu'ils ne sont pas exclus par la configuration

## Ressources

- [Conventional Commits](https://www.conventionalcommits.org/)
- [Keep a Changelog](https://keepachangelog.com/)
- [Semantic Versioning](https://semver.org/)

## Prochaines étapes

- Consultez le [Guide d'utilisation](Guide-d-utilisation) pour utiliser RepoLens
- Découvrez les [Presets](Presets) disponibles
- Explorez les [Catégories de règles](Categories-de-regles)
