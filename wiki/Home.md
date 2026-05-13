<!-- Auto-generated header - Do not edit manually -->
![Version](https://img.shields.io/badge/version-local-gray)

---

# RepoLens - Documentation

Bienvenue dans la documentation de RepoLens, un outil CLI pour auditer les dépôts GitHub et garantir le respect des bonnes pratiques, de la sécurité et de la conformité.

## Qu'est-ce que RepoLens ?

RepoLens est un outil en ligne de commande écrit en Rust qui permet d'auditer automatiquement vos dépôts GitHub pour :

- 🔒 **Sécurité** : Détection de secrets exposés, audit de sécurité du code, protection des branches, validation des politiques de sécurité, fonctionnalités de sécurité GitHub (v1.3.0)
- 📋 **Conformité** : Vérification des fichiers requis (README, LICENSE, CONTRIBUTING, etc.)
- 📚 **Documentation** : Validation de la qualité et de la complétude de la documentation
- ⚙️ **CI/CD** : Validation des workflows GitHub Actions
- 🎯 **Qualité** : Standards de qualité de code avec vérification de la couverture de tests (≥80%)
- 📦 **Dépendances** : Vérification de la sécurité des dépendances (9 écosystèmes supportés) via OSV API et GitHub Advisories
- 🔧 **Git** : Hygiène Git (binaires volumineux, fichiers sensibles, gitattributes)
- 📊 **Métadonnées** : Vérification des métadonnées du dépôt (description, topics, URL, social preview) *(v1.4.0)*
- 🎫 **Issues/PRs** : Hygiène des issues et PRs (stale, labels, reviewers, drafts abandonnées) *(v1.4.0)*
- 📜 **Historique** : Qualité de l'historique Git (conventional commits, commits géants, signatures, force push) *(v1.4.0)*
- 🛠️ **Règles personnalisées** : Support des règles d'audit personnalisées via regex ou commandes shell

## Navigation

### Pour les Utilisateurs

- [Installation](Installation) - Comment installer RepoLens (binaires, Docker, Homebrew, Scoop, AUR)
- [Guide d'utilisation](Guide-d-utilisation) - Utilisation de base et exemples
- [Configuration](Configuration) - Configuration avancée et variables d'environnement
- [Presets](Presets) - Presets disponibles (opensource, enterprise, strict)
- [Catégories de règles](Categories-de-regles) - Détails des règles d'audit
- [Règles personnalisées](Custom-Rules) - Créer vos propres règles d'audit
- [Changelog Automatique](Changelog-Automatique) - Génération automatique du changelog
- [Bonnes pratiques](Bonnes-pratiques) - Recommandations et préconisations

### Distribution & CI/CD

- [Docker](../docs/docker.md) - Utilisation avec Docker
- [Intégration CI/CD](../docs/ci-cd-integration.md) - GitHub Actions, GitLab CI, Jenkins, CircleCI, Azure DevOps

### Pour les Développeurs

- [Développement](Developpement) - Guide de développement et contribution
- [Architecture](Architecture) - Architecture du projet
- [Contribution](Contribution) - Comment contribuer au projet

## Démarrage rapide

```bash
# Installation via Docker (recommandé)
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens plan

# Ou via Homebrew (macOS/Linux)
brew tap systm-d/repolens && brew install repolens

# Ou via cargo
cargo install repolens

# Initialisation
repolens init --preset opensource

# Audit
repolens plan

# Audit d'un autre répertoire
repolens -C /path/to/project plan

# Mode verbose avec timing
repolens plan -vv

# Application des correctifs (mode interactif ou automatique)
repolens apply --interactive
repolens apply --dry-run

# Générer un rapport JSON avec validation de schéma
repolens report --format json --schema --validate

# Comparer deux rapports d'audit
repolens compare --base-file before.json --head-file after.json

# Installer les git hooks (pre-commit + pre-push)
repolens install-hooks
```

Pour l'intégration CI/CD, utilisez l'Action GitHub officielle :

```yaml
- uses: systm-d/repolens-action@v1
  with:
    preset: opensource
```

## Fonctionnalités principales

### Audit & Sécurité
- ✅ Audit automatique des dépôts GitHub
- ✅ Détection de secrets et credentials exposés
- ✅ **Audit de sécurité du code** : Détection de code unsafe, analyse Semgrep, vérification des patterns dangereux
- ✅ **Protection des branches** : Vérification de la configuration de protection (SEC007-010)
- ✅ **Hygiène Git** : Détection des binaires volumineux, fichiers sensibles, gitattributes (GIT001-003)
- ✅ **Fonctionnalités de sécurité GitHub** *(v1.3.0)* : Vulnerability alerts, Dependabot, Secret scanning (SEC011-014)
- ✅ **Permissions Actions** *(v1.3.0)* : Audit des permissions de workflow et actions autorisées (SEC015-017)
- ✅ **Contrôle d'accès** *(v1.3.0)* : Collaborateurs, équipes, deploy keys, apps installées (TEAM, KEY, APP)
- ✅ **Infrastructure** *(v1.3.0)* : Webhooks et environments (HOOK001-003, ENV001-003)
- ✅ **CODEOWNERS** *(v1.3.0)* : Validation du fichier CODEOWNERS (CODE001-003)
- ✅ **Releases** *(v1.3.0)* : Audit des releases et tags signés (REL001-003)

### Dépendances
- ✅ **Scan multi-écosystèmes** : 9 écosystèmes supportés (Rust, Node.js, Python, Go, .NET, Ruby, Dart/Flutter, Swift, iOS)
- ✅ **Vulnérabilités** : Détection via OSV API et GitHub Advisories (DEP001-002)
- ✅ **Lock files** : Vérification de la présence des fichiers de verrouillage (DEP003)
- ✅ **Conformité des licences** : Vérification de la compatibilité des licences (LIC001-LIC004)

### CLI & Configuration
- ✅ **Variables d'environnement** : Configuration via `REPOLENS_*` (preset, verbose, token, etc.)
- ✅ **Option -C** : Audit d'un répertoire différent (`repolens -C /path/to/project plan`)
- ✅ **Mode verbose** : Timing détaillé par catégorie (`-v`, `-vv`, `-vvv`)
- ✅ **Messages d'erreur améliorés** : Suggestions et contexte pour résoudre les problèmes

### Qualité & Documentation
- ✅ Vérification des fichiers requis
- ✅ Validation des workflows GitHub Actions
- ✅ **Couverture de tests** : Vérification minimale de 80% avec quality gates configurables
- ✅ **Règles personnalisées** : Patterns regex ou commandes shell

### Outils
- ✅ Génération de plans d'action
- ✅ Application automatique des correctifs
- ✅ Formats de sortie multiples (Terminal, JSON, SARIF, Markdown, HTML)
- ✅ **Cache d'audit** : Invalidation SHA256 pour des audits plus rapides
- ✅ **Git hooks** : Pre-commit (secrets) et pre-push (audit complet)
- ✅ **Comparaison de rapports** : Détection des régressions et améliorations
- ✅ **JSON Schema** : Schéma (draft-07) pour valider les rapports
- ✅ **Changelog automatique** : Génération à partir des commits

### Stabilité & Sécurité
- ✅ **Sécurité des dépendances** : Toutes les vulnérabilités connues corrigées
- ✅ **Permissions sécurisées** : `.repolens.toml` protégé avec chmod 600 sur Unix
- ✅ **Codes de sortie standardisés** : 0=succès, 1=critique, 2=warning, 3=erreur, 4=args invalides
- ✅ **Validation des entrées** : Les catégories et presets invalides génèrent un avertissement
- ✅ **1000+ tests** : Couverture complète du code

### Distribution
- ✅ **Docker** : Image officielle multi-architecture (amd64, arm64)
- ✅ **Gestionnaires de paquets** : Homebrew, Scoop, AUR, Debian
- ✅ **Intégration CI/CD** : GitHub Actions, GitLab CI, Jenkins, CircleCI, Azure DevOps

## Support

- 📖 Consultez la documentation complète ci-dessous
- 🐛 [Signaler un bug](https://github.com/systm-d/repolens/issues)
- 💡 [Proposer une fonctionnalité](https://github.com/systm-d/repolens/issues)
- 📧 Questions ? Ouvrez une issue sur GitHub
