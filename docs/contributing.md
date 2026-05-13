<!-- Auto-generated header - Do not edit manually -->
![Version](https://img.shields.io/badge/version-local-gray)

---

# Guide de Contribution

Merci de votre interet pour contribuer a RepoLens ! Ce guide complet vous accompagne pas a pas pour contribuer efficacement au projet.

## Table des matieres

- [Configuration de l'environnement](#configuration-de-lenvironnement)
- [Workflow de contribution](#workflow-de-contribution)
- [Standards de code Rust](#standards-de-code-rust)
- [Tests](#tests)
- [Processus de review](#processus-de-review)
- [Exemples pratiques](#exemples-pratiques)
- [Types de contributions](#types-de-contributions)
- [Code de conduite](#code-de-conduite)

---

## Configuration de l'environnement

### Prerequis systeme

| Outil | Version | Installation |
|-------|---------|--------------|
| **Rust** | 1.74+ (stable) | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **Git** | 2.x | Via gestionnaire de paquets |
| **GitHub CLI** | Optionnel | `gh auth login` pour tester les fonctionnalites GitHub |

### Verifier votre installation Rust

```bash
# Verifier la version de Rust
rustc --version
# Devrait afficher rustc 1.74.0 ou superieur

# Verifier cargo
cargo --version

# Mettre a jour Rust si necessaire
rustup update stable
```

### Cloner et configurer le projet

```bash
# 1. Fork le repository sur GitHub (via l'interface web)
# 2. Cloner votre fork
git clone https://github.com/VOTRE_USERNAME/cli--repolens.git
cd cli--repolens

# 3. Ajouter le repository upstream
git remote add upstream https://github.com/systm-d/repolens.git

# 4. Verifier les remotes
git remote -v
# origin    https://github.com/VOTRE_USERNAME/cli--repolens.git (fetch)
# origin    https://github.com/VOTRE_USERNAME/cli--repolens.git (push)
# upstream  https://github.com/systm-d/repolens.git (fetch)
# upstream  https://github.com/systm-d/repolens.git (push)
```

### Premiere compilation

```bash
# Compiler le projet
cargo build

# Verifier que tout fonctionne
cargo run -- --help

# Lancer les tests
cargo test
```

### Configuration de l'editeur (recommande)

#### VS Code

Installez l'extension **rust-analyzer** pour :
- Autocompletion
- Navigation dans le code
- Affichage des erreurs en temps reel
- Formatage automatique

Configuration recommandee (`.vscode/settings.json`) :

```json
{
    "rust-analyzer.checkOnSave.command": "clippy",
    "editor.formatOnSave": true,
    "[rust]": {
        "editor.defaultFormatter": "rust-lang.rust-analyzer"
    }
}
```

#### Autres editeurs

- **IntelliJ/CLion** : Plugin Rust
- **Vim/Neovim** : rust.vim + coc-rust-analyzer
- **Emacs** : rust-mode + lsp-mode

---

## Workflow de contribution

### Etape 1 : Synchroniser avec upstream

Avant de commencer, assurez-vous d'avoir la derniere version :

```bash
# Recuperer les derniers changements
git fetch upstream

# Se placer sur main et mettre a jour
git checkout main
git merge upstream/main

# Pousser les mises a jour sur votre fork
git push origin main
```

### Etape 2 : Creer une branche

Utilisez une convention de nommage claire :

```bash
# Pour une nouvelle fonctionnalite
git checkout -b feature/nom-de-la-feature

# Pour une correction de bug
git checkout -b fix/description-du-bug

# Pour de la documentation
git checkout -b docs/sujet-documente

# Pour du refactoring
git checkout -b refactor/zone-refactoree
```

**Exemples concrets :**
- `feature/add-yaml-output-format`
- `fix/secret-detection-false-positive`
- `docs/improve-cli-help`
- `refactor/simplify-rule-engine`

### Etape 3 : Developper

1. **Ecrivez votre code** en suivant les [standards de code](#standards-de-code-rust)
2. **Testez regulierement** pendant le developpement
3. **Commitez souvent** avec des messages clairs

### Etape 4 : Verifier avant le commit

Executez la verification complete :

```bash
# Script de verification complet
cargo check && \
cargo fmt --all -- --check && \
cargo clippy -- -D warnings && \
cargo test
```

Si tout passe, vous etes pret a commiter.

### Etape 5 : Commit avec messages conventionnels

RepoLens utilise les [Conventional Commits](https://www.conventionalcommits.org/) :

```bash
git add .
git commit -m "type(scope): description courte"
```

#### Types de commits

| Type | Description | Exemple |
|------|-------------|---------|
| `feat` | Nouvelle fonctionnalite | `feat(rules): add license detection rule` |
| `fix` | Correction de bug | `fix(secrets): reduce false positives for API keys` |
| `docs` | Documentation | `docs(readme): add installation instructions` |
| `refactor` | Refactoring sans changement fonctionnel | `refactor(engine): simplify rule execution flow` |
| `test` | Ajout ou modification de tests | `test(scanner): add edge case tests` |
| `chore` | Maintenance, dependencies | `chore(deps): update clap to 4.5` |
| `perf` | Amelioration de performance | `perf(scan): parallelize file scanning` |
| `style` | Formatage, style de code | `style: apply rustfmt` |

#### Bonnes pratiques pour les messages

```bash
# Bon : clair et specifique
git commit -m "feat(output): add SARIF format support for CI integration"

# Bon : avec scope explicite
git commit -m "fix(rules/secrets): ignore test files in secret detection"

# Mauvais : trop vague
git commit -m "fix stuff"

# Mauvais : pas de type
git commit -m "added new feature"
```

### Etape 6 : Push et Pull Request

```bash
# Pousser votre branche
git push origin feature/votre-feature
```

Ensuite, creez une Pull Request sur GitHub avec :
- **Titre clair** suivant la convention de commit
- **Description detaillee** du changement
- **Reference a l'issue** si applicable (`Fixes #123`)
- **Screenshots** si changement visuel

---

## Standards de code Rust

### Formatage avec rustfmt

Le formatage est obligatoire et verifie en CI :

```bash
# Verifier le formatage (sans modifier)
cargo fmt --all -- --check

# Formater automatiquement tout le code
cargo fmt --all
```

Le projet utilise la configuration par defaut de rustfmt. Ne modifiez pas `rustfmt.toml`.

### Linting avec Clippy

Clippy detecte les erreurs courantes et suggere des ameliorations :

```bash
# Lancer clippy
cargo clippy

# Avec warnings comme erreurs (comme en CI)
cargo clippy -- -D warnings

# Corriger automatiquement certains problemes
cargo clippy --fix
```

#### Annotations clippy acceptees

Parfois, il est necessaire de desactiver une regle clippy. Documentez toujours pourquoi :

```rust
// Acceptable : raison documentee
#[allow(clippy::too_many_arguments)]
// Cette fonction a besoin de tous ces parametres pour la configuration complete
fn complex_initialization(/* ... */) { }

// Non acceptable : pas de justification
#[allow(clippy::too_many_arguments)]
fn some_function(/* ... */) { }
```

### Conventions de nommage

| Element | Convention | Exemple |
|---------|------------|---------|
| Variables, fonctions | `snake_case` | `file_path`, `scan_directory()` |
| Types, Traits, Enums | `PascalCase` | `AuditResult`, `RuleCategory` |
| Constantes | `SCREAMING_SNAKE_CASE` | `MAX_FILE_SIZE`, `DEFAULT_TIMEOUT` |
| Modules | `snake_case` | `rule_engine`, `secret_detection` |
| Fichiers | `snake_case.rs` | `action_planner.rs` |

### Documentation du code

Documentez toutes les fonctions et structures publiques :

```rust
/// Scanne un repertoire a la recherche de fichiers correspondant aux patterns.
///
/// # Arguments
///
/// * `path` - Chemin du repertoire a scanner
/// * `patterns` - Liste de patterns glob a matcher
///
/// # Returns
///
/// Liste des chemins de fichiers trouves.
///
/// # Errors
///
/// Retourne une erreur si le repertoire n'existe pas ou n'est pas accessible.
///
/// # Example
///
/// ```rust
/// let files = scan_directory("/path/to/repo", &["*.rs", "*.toml"])?;
/// ```
pub fn scan_directory(path: &Path, patterns: &[&str]) -> Result<Vec<PathBuf>> {
    // Implementation
}
```

### Gestion des erreurs

Utilisez `anyhow` pour les erreurs applicatives et `thiserror` pour les erreurs typees :

```rust
use anyhow::{Context, Result};
use thiserror::Error;

// Erreur typee avec thiserror
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    NotFound(PathBuf),

    #[error("Invalid TOML syntax: {0}")]
    ParseError(#[from] toml::de::Error),
}

// Utilisation avec anyhow pour le contexte
pub fn load_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config: Config = toml::from_str(&content)
        .context("Failed to parse configuration")?;

    Ok(config)
}
```

### Conventions de logging

RepoLens distingue deux types de sortie :

#### Output utilisateur (println!/eprintln!)

```rust
use colored::Colorize;

// Progression
eprintln!("{}", "Analyse du depot...".dimmed());

// Succes
eprintln!("{} {}", "OK".green(), "Audit termine.".green());

// Warning
eprintln!("{} {}", "Warning:".yellow(), message);

// Erreur
eprintln!("{} {}", "Error:".red(), message);

// Resultat final (stdout)
println!("{}", rendered_output);
```

#### Logging structure (tracing)

```rust
// Debug - visible avec -v
tracing::debug!("Scanning {} files", file_count);

// Info - visible avec -v
tracing::info!("Cache loaded: {} entries", count);

// Warn - problemes non bloquants
tracing::warn!("Failed to parse optional config: {}", e);

// Trace - tres verbeux, visible avec -vvv
tracing::trace!("Processing file: {}", path.display());
```

**Regle importante** : Ne melangez jamais `tracing` et `eprintln!` pour le meme type de message.

---

## Tests

### Structure des tests

```
tests/
├── integration_test.rs  # Tests d'integration CLI
├── e2e_test.rs          # Tests end-to-end complets
├── providers_test.rs    # Tests des providers (GitHub)
├── regression_test.rs   # Tests de non-regression
├── security_test.rs     # Tests de securite
└── utils_test.rs        # Tests des utilitaires
```

### Lancer les tests

```bash
# Tous les tests
cargo test

# Tests avec output visible
cargo test -- --nocapture

# Test specifique par nom
cargo test test_scan_directory

# Tests d'un module
cargo test rules::

# Tests d'integration uniquement
cargo test --test integration_test

# Tests en sequence (pas en parallele)
cargo test -- --test-threads=1
```

### Ecrire des tests unitaires

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_finds_rust_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        std::fs::write(&file_path, "fn main() {}").unwrap();

        let results = scan_directory(temp_dir.path(), &["*.rs"]).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], file_path);
    }

    #[test]
    fn test_scan_empty_directory() {
        let temp_dir = tempfile::tempdir().unwrap();

        let results = scan_directory(temp_dir.path(), &["*.rs"]).unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_scan_nonexistent_directory_returns_error() {
        let result = scan_directory(Path::new("/nonexistent"), &["*.rs"]);

        assert!(result.is_err());
    }
}
```

### Tests d'integration

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("repolens").unwrap();

    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("audit"));
}

#[test]
fn test_cli_init_creates_config() {
    let temp_dir = tempfile::tempdir().unwrap();

    let mut cmd = Command::cargo_bin("repolens").unwrap();
    cmd.current_dir(temp_dir.path())
        .arg("init")
        .arg("--preset")
        .arg("opensource")
        .assert()
        .success();

    assert!(temp_dir.path().join(".repolens.toml").exists());
}
```

### Couverture de code

```bash
# Installer tarpaulin
cargo install cargo-tarpaulin

# Generer le rapport de couverture
cargo tarpaulin --out Html

# Le rapport est dans coverage/tarpaulin-report.html
```

---

## Processus de review

### Soumission de la Pull Request

1. **Creez la PR** avec un titre clair
2. **Remplissez le template** de PR
3. **Assurez-vous que la CI passe** (tests, fmt, clippy)

### Criteres d'acceptation

Votre PR sera acceptee si :

- [ ] Les tests passent (CI verte)
- [ ] Le code est formate (`cargo fmt`)
- [ ] Pas de warnings clippy
- [ ] Documentation a jour pour les changements publics
- [ ] Tests ajoutes pour les nouvelles fonctionnalites
- [ ] Pas de regression de performance significative
- [ ] Code lisible et maintenable
- [ ] Commits atomiques avec messages clairs

### Checklist avant soumission

```markdown
## Checklist

- [ ] J'ai lu le guide de contribution
- [ ] Mon code suit les standards de style
- [ ] J'ai ajoute des tests pour mes changements
- [ ] Tous les tests passent localement
- [ ] J'ai mis a jour la documentation si necessaire
- [ ] Mes commits suivent la convention
- [ ] J'ai verifie qu'il n'y a pas de secrets dans le code
```

### Deroulement de la review

1. **Soumission** : Vous creez la PR
2. **CI** : Les checks automatiques s'executent
3. **Review** : Un mainteneur examine le code
4. **Feedback** : Des commentaires peuvent etre laisses
5. **Iterations** : Vous apportez les modifications demandees
6. **Approbation** : Une fois approuvee, la PR est mergee

### Repondre aux commentaires

- Repondez a chaque commentaire
- Si vous n'etes pas d'accord, expliquez votre point de vue
- Marquez les conversations resolues une fois traitees
- Demandez des clarifications si necessaire

---

## Exemples pratiques

### Exemple 1 : Corriger un bug (bugfix)

**Scenario** : La detection de secrets genere un faux positif pour les cles de test.

```bash
# 1. Creer la branche
git checkout -b fix/secret-detection-test-keys

# 2. Localiser le code concerne
# src/rules/patterns/secrets.rs

# 3. Ajouter un test qui reproduit le bug
```

```rust
// tests/regression_test.rs
#[test]
fn test_no_false_positive_for_test_keys() {
    let content = r#"api_key = "test_key_for_unit_tests""#;
    let findings = detect_secrets(content);
    assert!(findings.is_empty(), "Test keys should not be flagged");
}
```

```bash
# 4. Verifier que le test echoue (reproduit le bug)
cargo test test_no_false_positive_for_test_keys
# Expected: FAILED

# 5. Corriger le code
# Modifier src/rules/patterns/secrets.rs pour ignorer les test keys

# 6. Verifier que le test passe maintenant
cargo test test_no_false_positive_for_test_keys
# Expected: PASSED

# 7. Verification complete
cargo check && cargo fmt --all -- --check && cargo clippy -- -D warnings && cargo test

# 8. Commit
git add .
git commit -m "fix(secrets): ignore test keys in secret detection

Test keys like 'test_key_*' are now excluded from secret detection
to reduce false positives in test files.

Fixes #42"

# 9. Push et PR
git push origin fix/secret-detection-test-keys
```

### Exemple 2 : Ajouter une fonctionnalite (feature)

**Scenario** : Ajouter le support du format de sortie YAML.

```bash
# 1. Creer la branche
git checkout -b feature/yaml-output-format

# 2. Ajouter la dependance si necessaire (deja presente: serde_yaml)
```

```rust
// src/cli/output/yaml.rs (nouveau fichier)
use crate::rules::AuditResults;
use anyhow::Result;

/// Formate les resultats d'audit en YAML.
pub fn format(results: &AuditResults) -> Result<String> {
    serde_yaml::to_string(results)
        .map_err(|e| anyhow::anyhow!("Failed to serialize to YAML: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_format_produces_valid_yaml() {
        let results = AuditResults::default();
        let yaml = format(&results).unwrap();

        // Verifier que c'est du YAML valide
        let _: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
    }
}
```

```rust
// src/cli/output/mod.rs - ajouter le module
pub mod yaml;

// Modifier la logique de selection du format
pub fn render(results: &AuditResults, format: &str) -> Result<String> {
    match format {
        "json" => json::format(results),
        "yaml" => yaml::format(results),  // Nouveau
        "terminal" => terminal::format(results),
        _ => Err(anyhow::anyhow!("Unknown format: {}", format)),
    }
}
```

```bash
# 3. Tests
cargo test yaml

# 4. Verification complete
cargo check && cargo fmt --all -- --check && cargo clippy -- -D warnings && cargo test

# 5. Commit
git add .
git commit -m "feat(output): add YAML output format

Adds support for YAML output format using the --format yaml flag.
This is useful for users who prefer YAML over JSON for configuration
and integration with other tools.

Usage: repolens report --format yaml"

# 6. Push et PR
git push origin feature/yaml-output-format
```

### Exemple 3 : Ameliorer la documentation

```bash
# 1. Creer la branche
git checkout -b docs/improve-cli-examples

# 2. Modifier la documentation
# Editer README.md, docs/, ou les docstrings dans le code

# 3. Verifier le build de la doc
cargo doc --no-deps --open

# 4. Commit
git add .
git commit -m "docs(readme): add practical CLI usage examples

Adds examples for common use cases:
- Running a basic audit
- Using presets
- Generating reports in different formats"

# 5. Push et PR
git push origin docs/improve-cli-examples
```

---

## Types de contributions

### Facile pour commencer

Ideal pour une premiere contribution :

- **Documentation** : Corriger des typos, clarifier des explications
- **Tests** : Ajouter des tests pour du code non couvert
- **Messages d'erreur** : Ameliorer la clarte des messages
- **Exemples** : Ajouter des exemples d'utilisation

### Niveau intermediaire

- **Nouvelles regles d'audit** : Ajouter des regles dans `src/rules/categories/`
- **Nouveaux formats de sortie** : YAML ou autres (voir spec recentering dans `docs/superpowers/specs/`)
- **Amelioration UX CLI** : Meilleurs messages, progress bars
- **Cache** : Ameliorer le systeme de cache (`src/cache/`)
- **Git hooks** : Nouveaux hooks (`src/hooks/`)

### Niveau avance

- **Optimisations de performance** : Profiling et optimisation
- **Nouveaux providers** : Support d'autres plateformes (GitLab, Bitbucket)
- **Module de comparaison** : Enrichir `src/compare/`
- **Nouvelles categories** : Licenses, dependencies, custom rules
- **Architecture** : Refactoring majeur, async improvements

---

## Questions et support

### Avant de poser une question

1. Consultez la [documentation](README.md)
2. Cherchez dans les [issues existantes](https://github.com/systm-d/repolens/issues)
3. Lisez le [guide de developpement](development.md)

### Poser une question

- Ouvrez une issue avec le label `question`
- Soyez precis sur votre probleme
- Incluez les versions (Rust, OS, RepoLens)
- Partagez les messages d'erreur complets

---

## Code de conduite

Nous suivons le [Contributor Covenant Code of Conduct](https://www.contributor-covenant.org/).

En bref :
- Soyez respectueux et inclusif
- Acceptez les critiques constructives
- Concentrez-vous sur ce qui est le mieux pour la communaute
- Faites preuve d'empathie envers les autres membres

---

## Merci !

Votre contribution est precieuse. Chaque correction de bug, amelioration de documentation ou nouvelle fonctionnalite rend RepoLens meilleur pour tout le monde.

Si vous avez des questions sur ce guide ou le processus de contribution, n'hesitez pas a ouvrir une issue.
