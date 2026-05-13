<!-- Auto-generated header - Do not edit manually -->
![Version](https://img.shields.io/badge/version-local-gray)

---

# Bonnes pratiques

Ce guide présente les meilleures pratiques pour utiliser RepoLens et maintenir un dépôt GitHub de qualité.

## Workflow recommandé

### 1. Initialisation d'un nouveau projet

```bash
# 1. Initialiser RepoLens
repolens init --preset opensource

# 2. Vérifier la configuration
cat .repolens.toml

# 3. Lancer un audit initial
repolens plan -v

# 4. Prévisualiser les correctifs
repolens apply --dry-run

# 5. Appliquer les correctifs
repolens apply
```

### 2. Installer les Git hooks

```bash
# Installer les hooks pre-commit et pre-push
repolens install-hooks

# Le hook pre-commit vérifie les secrets avant chaque commit
# Le hook pre-push lance un audit complet avant chaque push
```

### 3. Maintenance continue

```bash
# Intégrer dans votre workflow CI/CD
# Voir la section CI/CD ci-dessous
```

## Intégration CI/CD

### GitHub Actions

Ajoutez RepoLens à vos workflows GitHub Actions :

```yaml
name: RepoLens Audit

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]
  schedule:
    - cron: '0 0 * * 0'  # Hebdomadaire

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Build RepoLens
        run: |
          git clone https://github.com/systm-d/repolens.git
          cd cli--repolens
          cargo build --release
          sudo cp target/release/repolens /usr/local/bin/
      
      - name: Run audit
        run: |
          repolens plan --format sarif --output repolens-results.sarif
      
      - name: Upload SARIF
        uses: github/codeql-action/upload-sarif@v2
        with:
          sarif_file: repolens-results.sarif
```

### Pipeline local

```bash
#!/bin/bash
# Script de pré-commit

# Lancer l'audit
repolens plan

# Si des problèmes critiques sont trouvés, arrêter
if [ $? -ne 0 ]; then
    echo "❌ Audit échoué. Corrigez les problèmes avant de commiter."
    exit 1
fi
```

## Gestion des secrets

### ❌ À éviter

```bash
# Ne jamais faire ça
echo "api_key=sk_live_1234567890" >> config.json
git add config.json
git commit -m "Add config"
```

### ✅ Bonnes pratiques

1. **Utiliser des variables d'environnement**

```bash
# .env (ne pas commiter)
API_KEY=sk_live_1234567890

# .env.example (commiter)
API_KEY=sk_test_your_key_here
```

2. **Utiliser des gestionnaires de secrets**

- HashiCorp Vault
- AWS Secrets Manager
- GitHub Secrets (pour CI/CD)

3. **Vérifier avant de commiter**

```bash
# Lancer RepoLens avant chaque commit
repolens plan --only secrets
```

## Documentation

### README.md

Un bon README doit contenir :

- ✅ Description du projet
- ✅ Installation
- ✅ Utilisation de base
- ✅ Exemples de code
- ✅ Configuration
- ✅ Contribution
- ✅ Licence

### Exemple de structure

```markdown
# Mon Projet

Description courte du projet.

## Installation

```bash
npm install mon-projet
```

## Utilisation

```javascript
import { MonProjet } from 'mon-projet';

const instance = new MonProjet();
```

## Configuration

Voir [Configuration](docs/configuration.md)

## Contribution

Voir [CONTRIBUTING.md](CONTRIBUTING.md)

## Licence

MIT - Voir [LICENSE](LICENSE)
```

## Protection des branches

### Configuration recommandée

```toml
[actions.branch_protection]
enabled = true
branch = "main"
required_approvals = 1        # Au moins 1 review
require_status_checks = true  # Tests doivent passer
block_force_push = true       # Pas de force push
require_signed_commits = false # Optionnel selon le contexte
```

### Pour les projets critiques

```toml
[actions.branch_protection]
required_approvals = 2
require_signed_commits = true
```

## Gestion des dépendances

### Vérification de la sécurité des dépendances

RepoLens vérifie automatiquement les vulnérabilités dans vos dépendances via :

- **OSV API** : Base de données open-source des vulnérabilités
- **GitHub Security Advisories** : Base de données GitHub

#### Support multi-écosystèmes

- **Rust** : Analyse de `Cargo.lock`
- **Node.js** : Analyse de `package-lock.json`
- **Python** : Analyse de `requirements.txt`
- **Go** : Analyse de `go.sum`

### Vérifications régulières

```bash
# Vérifier les vulnérabilités dans les dépendances
repolens plan --only dependencies

# Vérifier la sécurité globale
repolens plan --only security,dependencies

# Utiliser Dependabot (GitHub)
# Activer dans les paramètres du dépôt
```

### Mises à jour

- ✅ Mettre à jour régulièrement les dépendances
- ✅ Tester après chaque mise à jour
- ✅ Utiliser des versions fixes (lock files)
- ✅ Vérifier les vulnérabilités avant chaque release
- ✅ Configurer Dependabot pour les mises à jour automatiques
- ✅ Surveiller les alertes de sécurité GitHub

## Tests

### Structure recommandée

```
tests/
├── unit/          # Tests unitaires
├── integration/   # Tests d'intégration
└── fixtures/      # Données de test
```

### Couverture de code

RepoLens exige une **couverture minimale de 80%** pour garantir la qualité du code.

#### Configuration des quality gates

Les seuils de qualité sont configurables dans `.github/quality-gates.toml` :

```toml
[coverage]
minimum = 80.0  # Couverture minimale requise
target = 90.0   # Objectif de couverture
exclude = [
    "src/main.rs",  # Point d'entrée, souvent difficile à tester
    "src/lib.rs",   # Fichier de réexport
    "tests/**",     # Tests eux-mêmes
]
```

#### Vérification de la couverture

```bash
# Générer un rapport de couverture
cargo tarpaulin --out Xml --output-dir coverage

# Vérifier les quality gates
cargo run --bin check-quality-gates
```

#### Intégration CI/CD

La couverture est vérifiée automatiquement dans les workflows GitHub Actions :

- Job `coverage` dans `.github/workflows/ci.yml`
- Vérification des quality gates dans `.github/workflows/nightly.yml`
- Upload des rapports vers Codecov (optionnel)

### Bonnes pratiques pour la couverture

- ✅ **Viser au moins 80% de coverage** (exigence minimale)
- ✅ Tester les cas limites et les cas d'erreur
- ✅ Tester les fonctions publiques de manière exhaustive
- ✅ Utiliser des tests d'intégration pour les workflows complexes
- ✅ Exclure les fichiers non testables (main.rs, lib.rs) du calcul
- ✅ Surveiller l'évolution de la couverture dans le temps
- ✅ Utiliser des outils comme `cargo-tarpaulin` pour Rust

## Code Review

### Checklist

- [ ] Code lisible et bien documenté
- [ ] Tests ajoutés/modifiés
- [ ] Pas de secrets exposés
- [ ] Documentation mise à jour
- [ ] Pas de warnings de linter
- [ ] Tests passent

## Sécurité

### Audit de sécurité du code

RepoLens effectue un audit complet de sécurité incluant :

- **Détection de code unsafe** : Recherche de blocs `unsafe` dans le code de production
- **Vérification des patterns dangereux** : Détection de patterns pouvant causer des vulnérabilités
- **Analyse avec Semgrep** : Intégration avec Semgrep pour détecter les vulnérabilités OWASP
- **Vérification des secrets** : Détection des secrets exposés
- **Vérification des dépendances** : Scan des vulnérabilités dans les dépendances

### Checklist de sécurité

- [ ] Aucun secret dans le code
- [ ] SECURITY.md présent
- [ ] Alertes de vulnérabilité activées
- [ ] Dependabot configuré
- [ ] Branches protégées
- [ ] Reviews de code obligatoires (CODEOWNERS)
- [ ] Aucun code `unsafe` dans le code de production
- [ ] Fichiers de verrouillage des dépendances présents
- [ ] Analyse de sécurité automatisée (Semgrep/CodeQL)
- [ ] Vérification régulière des vulnérabilités des dépendances

## Comparaison de rapports

### Suivi des améliorations

```bash
# Générer un rapport de référence (baseline)
repolens report --format json --output baseline.json

# Après avoir fait des corrections, générer un nouveau rapport
repolens report --format json --output current.json

# Comparer les deux rapports
repolens compare --base-file baseline.json --head-file current.json
```

### Intégration CI avec détection de régression

```bash
# Échouer le build si de nouveaux problèmes apparaissent
repolens compare --base-file baseline.json --head-file current.json --fail-on-regression
```

## Validation JSON Schema

### Valider les rapports JSON

```bash
# Générer un rapport avec validation automatique
repolens report --format json --schema --validate

# Exporter le schéma pour validation externe
repolens schema --output schemas/audit-report.schema.json
```

## Cache d'audit

### Bonnes pratiques de cache

- Utiliser le cache en développement local pour des audits rapides
- Utiliser `--no-cache` en CI pour des résultats toujours frais
- Utiliser `--clear-cache` après des changements de configuration
- Le cache est automatiquement invalidé quand le contenu d'un fichier change (hash SHA256)

```bash
# Développement local (cache activé par défaut)
repolens plan

# CI/CD (désactiver le cache)
repolens plan --no-cache
```

## Conformité des licences

### Configurer la vérification des licences

```toml
["rules.licenses"]
enabled = true
allowed_licenses = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC"]
denied_licenses = ["GPL-3.0", "AGPL-3.0"]
```

- Définir une liste blanche de licences autorisées pour vos dépendances
- Bloquer les licences incompatibles avec votre projet
- Surveiller les dépendances sans licence

## Performance

### Optimisations

- Utiliser le cache pour les audits répétitifs (activé par défaut)
- Utiliser `--dry-run` avant `apply`
- Filtrer par catégories si nécessaire
- Utiliser des presets appropriés

## Maintenance

### Audit régulier

```bash
# Audit hebdomadaire
repolens plan --format html --output weekly-audit.html

# Audit mensuel complet
repolens plan -vv --format json --output monthly-audit.json
```

### Mise à jour de RepoLens

```bash
# Mettre à jour depuis les sources
cd cli--repolens
git pull
cargo build --release
```

## Dépannage

### Problèmes courants

1. **Erreur de configuration**

```bash
# Vérifier la syntaxe
repolens init --validate
```

2. **Résultats inattendus**

```bash
# Mode debug
repolens plan -vvv
```

3. **Performance lente**

```bash
# Filtrer par catégories
repolens plan --only secrets,files
```

## Ressources

- [Guide d'utilisation](Guide-d-utilisation)
- [Configuration](Configuration)
- [Presets](Presets)
- [Catégories de règles](Categories-de-regles)

## Prochaines étapes

- Intégrez RepoLens dans votre workflow
- Configurez les presets selon vos besoins
- Automatisez les audits avec CI/CD
