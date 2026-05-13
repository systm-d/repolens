# Guide de Développement - RepoLens

Ce document explique comment développer, tester et contribuer au projet RepoLens.

## Prérequis

- **Rust** : Version stable (1.70+ recommandée)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Git** : Pour la gestion de version (requis)
- **GitHub CLI** (`gh`) : Requis pour les fonctionnalités GitHub
  ```bash
  # Installation : https://cli.github.com/
  gh auth login
  ```

> **Note** : La commande `repolens init` vérifie automatiquement ces prérequis. Utilisez `--skip-checks` pour ignorer cette vérification pendant le développement.

## Installation et Setup

1. **Cloner le repository**
   ```bash
   git clone https://github.com/systm-d/repolens.git
   cd repolens
   ```

2. **Installer les dépendances**
   ```bash
   cargo build
   ```

3. **Installer les git hooks**
   ```bash
   ./scripts/install-hooks.sh
   ```
   Les hooks installés :
   - **pre-commit** : Vérifie le formatage (`cargo fmt`) et le linting (`cargo clippy`)
   - **commit-msg** : Valide le format des messages de commit (Conventional Commits)

4. **Vérifier l'installation**
   ```bash
   cargo run -- --help
   ```

## Structure du Projet

```
src/
├── main.rs              # Point d'entrée du CLI
├── lib.rs               # Exports de la bibliothèque
├── cli/                 # Commandes CLI
│   ├── commands/        # Implémentation des commandes (init, plan, apply, report)
│   └── output/          # Formats de sortie (terminal, JSON, SARIF, Markdown, HTML)
├── config/              # Chargement et gestion de la configuration
│   └── presets/         # Presets de configuration (opensource, enterprise, strict)
├── rules/               # Moteur d'audit et règles
│   ├── categories/      # Catégories de règles (secrets, files, docs, security, workflows, quality)
│   ├── patterns/        # Patterns de détection (secrets, etc.)
│   └── engine.rs        # Moteur d'exécution des règles
├── actions/             # Planification et exécution des actions
│   ├── planner.rs       # Planification des actions à partir des résultats
│   ├── executor.rs      # Exécution des actions
│   └── templates.rs     # Génération de fichiers à partir de templates
├── providers/           # Intégration avec les APIs externes
│   └── github.rs        # Provider GitHub (via gh CLI)
├── scanner/             # Scan du système de fichiers et Git
│   ├── filesystem.rs    # Scan du système de fichiers
│   └── git.rs           # Informations Git
└── utils/               # Utilitaires partagés
    └── prerequisites.rs # Vérification des prérequis (git, gh, etc.)
```

## Commandes de Développement

### Compilation

```bash
# Compilation en mode debug (rapide, avec symboles de debug)
cargo build

# Compilation en mode release (optimisé)
cargo build --release

# Vérification sans compilation
cargo check
```

### Exécution en Mode Développement

```bash
# Lancer le CLI avec des arguments
cargo run -- --help
cargo run -- init
cargo run -- plan -vv
cargo run -- apply --dry-run

# Avec logs détaillés
cargo run -- plan -vvv  # Trace level
```

### Tests

```bash
# Lancer tous les tests
cargo test

# Tests avec output détaillé
cargo test -- --nocapture

# Tests d'un module spécifique
cargo test --lib rules

# Tests d'intégration
cargo test --test integration_test
```

### Linting et Formatage

```bash
# Vérifier le formatage
cargo fmt --all -- --check

# Formater le code
cargo fmt --all

# Linter avec clippy
cargo clippy

# Clippy avec erreurs en warnings
cargo clippy -- -D warnings
```

### Vérifications Complètes

```bash
# Vérification complète avant commit
cargo check && cargo fmt --all -- --check && cargo clippy -- -D warnings && cargo test
```

## Architecture

### Flux d'Exécution

1. **CLI** (`main.rs`) : Parse les arguments et route vers la commande appropriée
2. **Config** : Charge la configuration depuis `.repolens.toml` ou utilise un preset
3. **Scanner** : Scanne le repository (fichiers, Git)
4. **Rules Engine** : Exécute les règles d'audit par catégorie
5. **Action Planner** : Génère un plan d'actions basé sur les résultats
6. **Output** : Formate et affiche les résultats selon le format demandé

### Ajouter une Nouvelle Règle

1. **Créer la règle dans la catégorie appropriée** (`src/rules/categories/`)
   ```rust
   // src/rules/categories/ma_categorie.rs
   pub fn check_ma_regle(scanner: &Scanner, config: &Config) -> Vec<Finding> {
       let mut findings = Vec::new();
       // Logique de la règle
       findings
   }
   ```

2. **Enregistrer la règle dans le moteur** (`src/rules/engine.rs`)
   ```rust
   match category {
       "ma_categorie" => {
           findings.extend(ma_categorie::check_ma_regle(&scanner, &config)?);
       }
       // ...
   }
   ```

3. **Ajouter la catégorie dans la configuration** (`src/config/loader.rs`)

### Ajouter une Nouvelle Action

1. **Créer l'action** (`src/actions/`)
   ```rust
   pub struct MonAction {
       // Champs nécessaires
   }
   
   impl Action for MonAction {
       fn execute(&self) -> Result<ActionResult> {
           // Logique d'exécution
       }
   }
   ```

2. **Ajouter la planification** (`src/actions/planner.rs`)
   ```rust
   if condition {
       plan.add_action(Box::new(MonAction::new(...)));
   }
   ```

### Ajouter un Nouveau Format de Sortie

1. **Créer le module de sortie** (`src/cli/output/`)
   ```rust
   // src/cli/output/mon_format.rs
   pub fn format(results: &AuditResults) -> String {
       // Formatage
   }
   ```

2. **Enregistrer dans le module** (`src/cli/output/mod.rs`)

## Debugging

### Logs

Le projet utilise `tracing` pour les logs. Niveaux de verbosité :

- `-v` : `info` level
- `-vv` : `debug` level  
- `-vvv` : `trace` level

```bash
# Avec logs détaillés
cargo run -- plan -vv

# Avec variable d'environnement
RUST_LOG=debug cargo run -- plan
```

### Debug avec GDB/LLDB

```bash
# Compiler avec symboles
cargo build

# Lancer avec GDB
gdb target/debug/repolens

# Ou avec LLDB (macOS)
lldb target/debug/repolens
```

### Tests de Détection de Secrets

Les patterns de secrets sont dans `src/rules/patterns/secrets.rs`. Pour tester :

```bash
# Créer un fichier de test avec un faux secret
echo "api_key = sk_test_1234567890abcdef" > test_secret.txt

# Lancer l'audit
cargo run -- plan

# Nettoyer
rm test_secret.txt
```

## Configuration de Développement

### Fichier `.repolens.toml`

Créer un fichier `.repolens.toml` à la racine pour tester :

```toml
[general]
preset = "opensource"

[rules]
secrets = true
files = true
docs = true
security = true
workflows = true
quality = true
```

### Presets

Les presets sont dans `presets/` :
- `opensource.toml` : Standards open-source
- `enterprise.toml` : Sécurité entreprise
- `strict.toml` : Maximum de sécurité

## Tests d'Intégration

Les tests d'intégration sont dans `tests/integration_test.rs`. Ils testent le CLI complet :

```bash
# Lancer les tests d'intégration
cargo test --test integration_test
```

## Workflow de Contribution

1. **Créer une branche**
   ```bash
   git checkout -b feature/ma-feature
   ```

2. **Développer et tester**
   ```bash
   cargo check
   cargo test
   cargo fmt --all
   cargo clippy
   ```

3. **Tester le CLI**
   ```bash
   cargo run -- plan -vv
   cargo run -- apply --dry-run
   ```

4. **Commit et Push**
   ```bash
   git add .
   git commit -m "feat: ajout de ma feature"
   git push origin feature/ma-feature
   ```
   > Les hooks pre-commit vérifient automatiquement le formatage et le linting.

5. **Créer une Pull Request**

## Messages de Commit (Conventional Commits)

Les messages de commit doivent suivre le format [Conventional Commits](https://www.conventionalcommits.org/) :

```
<type>(<scope>): <description>

[corps optionnel]

[footer optionnel]
```

### Types disponibles

| Type | Description |
|------|-------------|
| `feat` | Nouvelle fonctionnalité |
| `fix` | Correction de bug |
| `docs` | Documentation |
| `style` | Formatage, style de code |
| `refactor` | Refactoring sans changement fonctionnel |
| `perf` | Amélioration des performances |
| `test` | Ajout ou modification de tests |
| `build` | Système de build, dépendances |
| `ci` | Configuration CI/CD |
| `chore` | Maintenance, tâches diverses |
| `revert` | Annulation d'un commit précédent |

### Exemples

```bash
git commit -m "feat: add user authentication"
git commit -m "fix(api): resolve null pointer exception"
git commit -m "docs: update README with installation instructions"
git commit -m "refactor(cli): simplify command parsing logic"
```

> Le hook `commit-msg` valide automatiquement le format des messages.

## Bonnes Pratiques

### Code Style

- Utiliser `cargo fmt` avant chaque commit (vérifié par le hook pre-commit)
- Respecter les conventions Rust (snake_case, etc.)
- Documenter les fonctions publiques avec `///`

### Gestion d'Erreurs

- Utiliser `anyhow::Result` pour les erreurs applicatives
- Utiliser `thiserror` pour les erreurs typées
- Toujours propager les erreurs avec `?`

### Performance

- Utiliser `async/await` pour les opérations I/O
- Éviter les allocations inutiles
- Profiler avec `cargo flamegraph` si nécessaire

### Tests

- Écrire des tests unitaires pour chaque fonction publique
- Ajouter des tests d'intégration pour les workflows complets
- Tester les cas limites et les erreurs

## Dépannage

### Erreurs de Compilation

```bash
# Nettoyer et reconstruire
cargo clean
cargo build
```

### Problèmes avec les Dépendances

```bash
# Mettre à jour les dépendances
cargo update

# Vérifier les versions
cargo tree
```

### Tests qui Échouent

```bash
# Lancer un test spécifique avec output
cargo test nom_du_test -- --nocapture

# Tests en parallèle (désactiver)
cargo test -- --test-threads=1
```

## Ressources

- [Documentation Rust](https://doc.rust-lang.org/)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Clap Documentation](https://docs.rs/clap/)
- [Tokio Documentation](https://tokio.rs/)

## Support

Pour toute question ou problème :
- Ouvrir une issue sur GitHub
- Consulter la documentation dans le code
- Vérifier les exemples dans `tests/`
