<!-- Auto-generated header - Do not edit manually -->
![Version](https://img.shields.io/badge/version-local-gray)

---

# Configuration

RepoLens utilise un fichier de configuration TOML (`.repolens.toml`) à la racine de votre projet pour personnaliser le comportement de l'audit.

## Structure de base

```toml
[general]
preset = "opensource"  # ou "enterprise", "strict"

[rules]
secrets = true
files = true
docs = true
security = true
workflows = true
quality = true
```

## Section `[general]`

### `preset`

Définit le preset à utiliser. Les presets sont des configurations prédéfinies :

- `opensource` : Standards open-source (par défaut)
- `enterprise` : Configuration entreprise
- `strict` : Sécurité maximale

```toml
[general]
preset = "opensource"
```

## Section `[rules]`

Active ou désactive les catégories de règles.

```toml
[rules]
secrets = true        # Détection de secrets
files = true          # Vérification des fichiers requis
docs = true           # Qualité de la documentation
security = true       # Bonnes pratiques de sécurité
workflows = true      # Validation des workflows GitHub Actions
quality = true        # Standards de qualité de code
licenses = true       # Conformité des licences (LIC001-LIC004)
dependencies = true   # Vulnérabilités des dépendances (DEP001-DEP002)
custom = true         # Règles personnalisées
```

## Configuration des secrets

### `[rules.secrets]`

```toml
[rules.secrets]
# Patterns à ignorer (chemins glob)
ignore_patterns = [
    "**/test/**",
    "**/tests/**",
    "**/__tests__/**",
    "**/*.test.*",
    "**/*.spec.*",
]

# Fichiers spécifiques à ignorer
ignore_files = [
    ".env.example",
    "config.example.json",
]
```

## Configuration des fichiers requis

### `[files.required]`

```toml
[files.required]
readme = true
license = true
contributing = true
code_of_conduct = true
security = true
```

## Configuration des actions

### `[actions]`

Définit quelles actions peuvent être exécutées automatiquement.

```toml
[actions]
gitignore = true  # Mettre à jour .gitignore automatiquement
```

### `[actions.license]`

```toml
[actions.license]
enabled = true
type = "MIT"  # MIT, Apache-2.0, GPL-3.0
# author = "Votre Nom"  # Optionnel
```

### `[actions.contributing]`

```toml
[actions.contributing]
enabled = true
```

### `[actions.code_of_conduct]`

```toml
[actions.code_of_conduct]
enabled = true
```

### `[actions.security_policy]`

```toml
[actions.security_policy]
enabled = true
```

### `[actions.branch_protection]`

```toml
[actions.branch_protection]
enabled = true
branch = "main"
required_approvals = 1
require_status_checks = true
block_force_push = true
require_signed_commits = false
```

### `[actions.github_settings]`

```toml
[actions.github_settings]
discussions = true
issues = true
wiki = false
vulnerability_alerts = true
automated_security_fixes = true
```

## Configuration des templates

### `[templates]`

Variables utilisées dans les templates générés.

```toml
[templates]
license_author = "Votre Nom"
license_year = "2025"
project_name = "Mon Projet"
project_description = "Description de mon projet"
```

## Exemples de configuration

### Configuration minimale

```toml
[general]
preset = "opensource"
```

### Configuration personnalisée

```toml
[general]
preset = "opensource"

[rules]
secrets = true
files = true
docs = true
security = true
workflows = false  # Désactiver la validation des workflows
quality = false    # Désactiver les vérifications de qualité

[rules.secrets]
ignore_patterns = [
    "**/test/**",
    "**/fixtures/**",
]

[files.required]
readme = true
license = true
contributing = false  # Pas de CONTRIBUTING requis
code_of_conduct = false
security = true

[actions]
gitignore = true

[actions.license]
enabled = true
type = "MIT"
author = "Mon Équipe"

[actions.branch_protection]
enabled = true
branch = "main"
required_approvals = 1
```

### Configuration entreprise

```toml
[general]
preset = "enterprise"

[rules]
secrets = true
files = true
docs = true
security = true
workflows = true
quality = true

[rules.secrets]
ignore_patterns = [
    "**/test/**",
    "**/tests/**",
    "**/fixtures/**",
    "**/mocks/**",
]

[actions.branch_protection]
enabled = true
branch = "main"
required_approvals = 2  # Plus strict pour l'entreprise
require_signed_commits = true
```

## Configuration des licences

### `["rules.licenses"]`

```toml
["rules.licenses"]
enabled = true
allowed_licenses = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC"]
denied_licenses = ["GPL-3.0", "AGPL-3.0"]
```

- `allowed_licenses` : Liste blanche de licences SPDX autorisées pour les dépendances
- `denied_licenses` : Liste noire de licences SPDX interdites

## Configuration du cache

### `[cache]`

```toml
[cache]
# Activer/désactiver le cache (défaut : true)
enabled = true
# Durée maximale des entrées de cache en heures (défaut : 24)
max_age_hours = 24
# Répertoire de cache (relatif à la racine du projet ou chemin absolu)
directory = ".repolens/cache"
```

Options CLI associées :
- `--no-cache` : Désactiver le cache pour un audit complet
- `--clear-cache` : Vider le cache avant l'audit
- `--cache-dir <DIR>` : Utiliser un répertoire de cache personnalisé

## Configuration des Git hooks

### `[hooks]`

```toml
[hooks]
# Installer le hook pre-commit (vérifie les secrets exposés)
pre_commit = true
# Installer le hook pre-push (lance un audit complet)
pre_push = true
# Échouer aussi sur les warnings (pas seulement les critiques)
fail_on_warnings = false
```

Installation via CLI :
```bash
repolens install-hooks              # Installer tous les hooks configurés
repolens install-hooks --pre-commit # Uniquement pre-commit
repolens install-hooks --pre-push   # Uniquement pre-push
repolens install-hooks --force      # Écraser les hooks existants (sauvegarde automatique)
repolens install-hooks --remove     # Supprimer les hooks RepoLens
```

## Priorité de configuration

L'ordre de priorité (du plus haut au plus bas) :

1. **Options CLI** : Flags passés en ligne de commande (ex: `--preset enterprise`)
2. **Variables d'environnement** : Variables `REPOLENS_*` (ex: `REPOLENS_PRESET=enterprise`)
3. **Fichier `.repolens.toml`** : Configuration locale du projet
4. **Preset** : Configuration du preset sélectionné
5. **Valeurs par défaut** : Valeurs par défaut de RepoLens

## Validation de la configuration

```bash
# Vérifier la syntaxe de la configuration
repolens plan --dry-run

# Ou avec validation explicite
repolens init --validate
```

## Variables d'environnement

RepoLens peut être configuré via des variables d'environnement. L'ordre de priorité est :
**CLI > Variables d'environnement > Fichier de configuration > Valeurs par défaut**

### Variables supportées

| Variable | Description | Valeurs | Exemple |
|----------|-------------|---------|---------|
| `REPOLENS_PRESET` | Preset par défaut | `opensource`, `enterprise`, `strict` | `enterprise` |
| `REPOLENS_VERBOSE` | Niveau de verbosité | `0` à `3` | `2` |
| `REPOLENS_CONFIG` | Chemin du fichier de configuration | Chemin absolu ou relatif | `/path/to/.repolens.toml` |
| `REPOLENS_NO_CACHE` | Désactiver le cache | `true`, `false`, `1`, `0` | `true` |
| `REPOLENS_GITHUB_TOKEN` | Token GitHub pour les appels API | Token `ghp_xxx` | `ghp_xxxxxxxxxxxx` |
| `GITHUB_TOKEN` | Token GitHub standard (v1.2.0+) | Token `ghp_xxx` | `ghp_xxxxxxxxxxxx` |

### Authentification GitHub

RepoLens supporte deux méthodes d'authentification GitHub (par ordre de priorité) :

1. **`GITHUB_TOKEN`** (recommandé) : Variable d'environnement standard GitHub, utilisée directement par l'API octocrab
2. **`gh auth login`** : Authentification via GitHub CLI (fallback automatique)

> **Note v1.2.0** : RepoLens utilise maintenant [octocrab](https://github.com/XAMPPRocky/octocrab) pour l'accès direct à l'API GitHub. Si `GITHUB_TOKEN` est défini, RepoLens n'a plus besoin de `gh` CLI installé.

### Exemples d'utilisation

```bash
# Configurer le preset par défaut
export REPOLENS_PRESET=enterprise

# Activer le mode verbose
export REPOLENS_VERBOSE=2

# Désactiver le cache
export REPOLENS_NO_CACHE=true

# Authentification GitHub (méthode recommandée v1.2.0+)
export GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx

# Ou via REPOLENS_GITHUB_TOKEN (compatibilité)
export REPOLENS_GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx

# Exécuter avec la configuration d'environnement
repolens plan
```

### Niveaux de verbosité

| Niveau | Description | Affichage |
|--------|-------------|-----------|
| `0` | Normal | Résultats uniquement |
| `1` | Basique (`-v`) | + Timing total |
| `2` | Détaillé (`-vv`) | + Timing par catégorie |
| `3` | Debug (`-vvv`) | + Informations de debug |

### Variables de debug

```bash
# Niveau de log Rust (pour le développement)
export RUST_LOG=debug

# Désactiver les couleurs
export NO_COLOR=1
```

## Sécurité du fichier de configuration

Le fichier `.repolens.toml` peut contenir des informations sensibles (patterns de secrets à ignorer, configuration personnalisée). Sur les systèmes Unix, RepoLens applique automatiquement les permissions `600` (lecture/écriture propriétaire uniquement) lors de la création du fichier via `repolens init`.

```bash
# Vérifier les permissions
ls -la .repolens.toml
# -rw-------  1 user  group  1234 Feb  7 10:00 .repolens.toml
```

> **Note** : Sur Windows, le système de permissions est différent et cette protection n'est pas appliquée automatiquement.

## Codes de sortie

RepoLens utilise des codes de sortie standardisés pour l'intégration CI/CD :

| Code | Constante | Signification |
|------|-----------|---------------|
| 0 | `SUCCESS` | Succès - pas de problèmes critiques |
| 1 | `CRITICAL_ISSUES` | Problèmes critiques détectés |
| 2 | `WARNINGS` | Avertissements détectés |
| 3 | `ERROR` | Erreur d'exécution |
| 4 | `INVALID_ARGS` | Arguments invalides |

```bash
# Utilisation dans un script CI/CD
repolens plan
EXIT_CODE=$?

if [ $EXIT_CODE -eq 0 ]; then
  echo "✅ Audit réussi"
elif [ $EXIT_CODE -eq 1 ]; then
  echo "❌ Problèmes critiques - blocage"
  exit 1
elif [ $EXIT_CODE -eq 2 ]; then
  echo "⚠️ Avertissements - revue recommandée"
fi
```

## Prochaines étapes

- Consultez les [Presets](presets.md) pour des configurations prédéfinies
- Découvrez les [Catégories de règles](rule-categories.md) pour comprendre chaque règle
