<!-- Auto-generated header - Do not edit manually -->
![Version](https://img.shields.io/badge/version-local-gray)

---

# Architecture

Ce document décrit l'architecture technique de RepoLens.

## Vue d'ensemble

RepoLens est construit en Rust avec une architecture modulaire qui sépare les préoccupations :

- **CLI** : Interface en ligne de commande
- **Config** : Gestion de la configuration
- **Scanner** : Analyse du dépôt
- **Rules Engine** : Moteur d'exécution des règles
- **Actions** : Planification et exécution des correctifs
- **Providers** : Intégration avec les APIs externes
- **Output** : Formats de sortie

## Architecture en couches

```
┌─────────────────────────────────────────┐
│           CLI Layer                     │
│  (main.rs, cli/commands/)              │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│        Business Logic                    │
│  (rules/, actions/, scanner/)           │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│        Infrastructure                   │
│  (config/, providers/, output/)         │
└─────────────────────────────────────────┘
```

## Structure des modules

```
src/
├── main.rs              # Point d'entrée
├── lib.rs               # Exports de la bibliothèque
├── cli/                 # Commandes CLI et formats de sortie
│   ├── commands/        # init, plan, apply, report, schema, compare, install_hooks
│   └── output/          # terminal, JSON, SARIF, Markdown, HTML
├── cache/               # Système de cache d'audit (invalidation SHA256)
├── compare/             # Comparaison de rapports (score diff, régressions, améliorations)
├── config/              # Chargement de configuration et presets
├── hooks/               # Gestion des Git hooks (pre-commit, pre-push)
├── rules/               # Moteur d'audit et règles
│   ├── categories/      # secrets, files, docs, security, workflows, quality, licenses, dependencies, custom
│   ├── patterns/        # Patterns de détection (secrets)
│   └── engine.rs        # Moteur d'exécution
├── actions/             # Planification et exécution des correctifs
├── providers/           # Intégration APIs externes (GitHub via gh CLI)
├── scanner/             # Scan du système de fichiers et Git
└── utils/               # Utilitaires (vérification des prérequis)
```

## Modules principaux

### CLI (`src/cli/`)

Gère l'interface en ligne de commande et le routage des commandes.

**Responsabilités** :
- Parsing des arguments (via `clap`)
- Routage vers les commandes appropriées
- Gestion de la sortie utilisateur

**Commandes** :
- `init` : Initialisation de la configuration
- `plan` : Génération du plan d'audit
- `apply` : Application des correctifs (mode interactif supporté)
- `report` : Génération de rapports (avec validation JSON Schema)
- `schema` : Affichage/export du JSON Schema pour les rapports d'audit
- `compare` : Comparaison de deux rapports d'audit JSON
- `install-hooks` : Installation/suppression des Git hooks (pre-commit, pre-push)

### Configuration (`src/config/`)

Gère le chargement et la validation de la configuration.

**Responsabilités** :
- Chargement depuis `.repolens.toml`
- Application des presets
- Validation de la configuration
- Fusion des configurations (local + preset + defaults)

**Fichiers** :
- `loader.rs` : Chargement de la configuration
- `presets/` : Définitions des presets

### Scanner (`src/scanner/`)

Analyse le dépôt pour collecter les informations nécessaires.

**Responsabilités** :
- Scan du système de fichiers
- Extraction d'informations Git
- Détection de fichiers et patterns

**Modules** :
- `filesystem.rs` : Scan des fichiers
- `git.rs` : Informations Git (branches, commits, etc.)

### Rules Engine (`src/rules/`)

Moteur d'exécution des règles d'audit.

**Responsabilités** :
- Exécution des règles par catégorie
- Collecte des findings
- Calcul de la sévérité

**Structure** :
- `engine.rs` : Moteur principal
- `categories/` : Catégories de règles
  - `secrets.rs` : Détection de secrets
  - `files.rs` : Vérification des fichiers
  - `docs.rs` : Qualité de la documentation
  - `security.rs` : Bonnes pratiques de sécurité
  - `workflows.rs` : Validation des workflows
  - `quality.rs` : Standards de qualité
  - `licenses.rs` : Conformité des licences (LIC001-LIC004)
  - `dependencies.rs` : Vulnérabilités des dépendances via OSV API (DEP001-DEP002)
  - `custom.rs` : Règles personnalisées (regex/shell)
- `patterns/` : Patterns de détection
  - `secrets.rs` : Patterns de secrets

### Actions (`src/actions/`)

Planification et exécution des actions correctives.

**Responsabilités** :
- Planification des actions basée sur les findings
- Exécution des actions
- Génération de fichiers depuis templates

**Modules** :
- `planner.rs` : Planification des actions
- `executor.rs` : Exécution des actions
- `templates.rs` : Génération depuis templates
- `github_settings.rs` : Configuration GitHub
- `branch_protection.rs` : Protection des branches
- `gitignore.rs` : Mise à jour de .gitignore

### Cache (`src/cache/`)

Système de mise en cache des résultats d'audit.

**Responsabilités** :
- Mise en cache des résultats d'audit avec hashing SHA256
- Invalidation automatique lors de changements de contenu
- Gestion de l'expiration (max_age_hours configurable)

**Options CLI** :
- `--no-cache` : Désactiver le cache
- `--clear-cache` : Vider le cache avant l'audit
- `--cache-dir` : Répertoire de cache personnalisé

### Compare (`src/compare/`)

Comparaison de rapports d'audit pour détecter les régressions et améliorations.

**Responsabilités** :
- Calcul du score pondéré (Critical=10, Warning=3, Info=1)
- Détection des nouveaux findings (régressions)
- Détection des findings résolus (améliorations)
- Ventilation par catégorie

**Formats de sortie** : Terminal (coloré), JSON, Markdown

### Hooks (`src/hooks/`)

Gestion des Git hooks pour l'intégration dans le workflow de développement.

**Responsabilités** :
- Génération et installation de hooks pre-commit (scan de secrets)
- Génération et installation de hooks pre-push (audit complet)
- Sauvegarde automatique des hooks existants
- Restauration des hooks originaux à la suppression

### Providers (`src/providers/`)

Intégration avec les APIs externes.

**Responsabilités** :
- Communication avec GitHub API
- Abstraction des APIs externes

**Modules** :
- `github.rs` : Provider GitHub (via `gh` CLI)

### Output (`src/cli/output/`)

Formats de sortie pour les résultats.

**Responsabilités** :
- Formatage des résultats
- Export dans différents formats

**Formats** :
- `terminal.rs` : Sortie terminal colorée
- `json.rs` : Format JSON
- `sarif.rs` : Format SARIF (pour GitHub Security)
- `markdown.rs` : Format Markdown
- `html.rs` : Format HTML

## Flux de données

### Commande `plan`

```
CLI (plan)
  ↓
Config Loader
  ↓
Scanner (scan filesystem + git)
  ↓
Rules Engine (execute rules)
  ↓
Action Planner (generate actions)
  ↓
Output Formatter
  ↓
Terminal/File
```

### Commande `apply`

```
CLI (apply)
  ↓
Config Loader
  ↓
Action Executor
  ↓
  ├─ Template Generator (create files)
  ├─ GitHub Provider (update settings)
  └─ Git Operations (update .gitignore)
  ↓
Results
```

## Patterns de conception

### Strategy Pattern

Les formats de sortie utilisent le pattern Strategy :

```rust
trait OutputFormatter {
    fn format(&self, results: &AuditResults) -> String;
}
```

### Factory Pattern

Les actions sont créées via un factory basé sur les findings :

```rust
impl ActionPlanner {
    fn plan_actions(&self, findings: &[Finding]) -> Vec<Box<dyn Action>> {
        // Création d'actions basée sur les findings
    }
}
```

### Provider Pattern

Les intégrations externes utilisent le pattern Provider :

```rust
trait Provider {
    async fn get_repo_info(&self) -> Result<RepoInfo>;
}
```

## Gestion des erreurs

### Types d'erreurs

- **ConfigError** : Erreurs de configuration
- **ScanError** : Erreurs de scan
- **RuleError** : Erreurs d'exécution de règles
- **ActionError** : Erreurs d'exécution d'actions
- **ProviderError** : Erreurs d'API externes

### Propagation

Utilisation de `anyhow::Result` pour la propagation d'erreurs avec contexte :

```rust
fn scan_repository() -> anyhow::Result<ScanResults> {
    let files = scan_files()?;
    let git_info = get_git_info()?;
    Ok(ScanResults { files, git_info })
}
```

## Performance

### Optimisations

- **Async I/O** : Utilisation de `tokio` pour les opérations I/O
- **Lazy evaluation** : Chargement à la demande
- **Caching** : Cache des résultats de scan
- **Parallel execution** : Exécution parallèle des règles indépendantes

### Profiling

```bash
# Profiler avec flamegraph
cargo install flamegraph
cargo flamegraph --bin repolens -- plan
```

## Extensibilité

### Ajouter une nouvelle règle

1. Créer la fonction de règle dans `src/rules/categories/`
2. Enregistrer dans `src/rules/engine.rs`
3. Ajouter la configuration dans `src/config/loader.rs`

### Ajouter un nouveau format

1. Créer le module dans `src/cli/output/`
2. Implémenter le trait `OutputFormatter`
3. Enregistrer dans `src/cli/output/mod.rs`

### Ajouter un nouveau provider

1. Créer le module dans `src/providers/`
2. Implémenter le trait `Provider`
3. Utiliser dans les actions appropriées

## Tests

### Structure des tests

```
tests/
├── unit/              # Tests unitaires (dans les modules)
├── integration_test.rs # Tests d'intégration
└── fixtures/          # Données de test
```

### Stratégie de test

- **Unit tests** : Chaque module a ses tests
- **Integration tests** : Tests du CLI complet
- **Mock providers** : Mocks pour les tests d'intégration

## Sécurité

### Considérations

- Validation stricte de la configuration
- Sanitization des inputs
- Pas d'exécution de code arbitraire
- Gestion sécurisée des secrets (jamais loggés)

## Dépendances principales

- **clap** : CLI framework
- **tokio** : Runtime async
- **serde** / **serde_json** : Sérialisation
- **toml** : Parsing TOML
- **regex** : Pattern matching
- **tracing** : Logging
- **tera** / **minijinja** : Templates
- **jsonschema** : Validation JSON Schema
- **reqwest** : Client HTTP (OSV API)
- **similar** : Calcul de diff (mode interactif)
- **dialoguer** : Prompts interactifs
- **indicatif** : Barres de progression
- **chrono** : Gestion date/heure pour les rapports

## Prochaines étapes

- Consultez le [Guide de Développement](development.md) pour commencer à contribuer
- Explorez le code source pour comprendre les détails d'implémentation
