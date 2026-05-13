<!-- Auto-generated header - Do not edit manually -->
![Version](https://img.shields.io/badge/version-local-gray)

---

# Règles personnalisées

## Security Considerations

> **Warning: Arbitrary Code Execution Risk**
>
> Custom rules that use the `command` parameter execute shell commands directly on your system. This is a powerful feature but comes with significant security risks:
>
> - **Only use commands from trusted sources.** A malicious `.repolens.toml` file could execute harmful commands on your system, including data exfiltration, file deletion, or installation of malware.
>
> - **Never commit untrusted `.repolens.toml` files.** Before running RepoLens on a cloned repository, always review the `.repolens.toml` file for any suspicious shell commands.
>
> - **Review before execution.** When opening a project with a `.repolens.toml` file that contains custom rules with shell commands, RepoLens will display a warning. Take this warning seriously and verify the commands before proceeding.
>
> Examples of dangerous commands to watch for:
> - Commands that download and execute scripts: `curl ... | sh`
> - Commands that modify system files or configurations
> - Commands that send data to external servers
> - Commands with obfuscated or encoded content

RepoLens permet de définir des règles d'audit personnalisées via la configuration TOML. Vous pouvez créer des règles basées sur des patterns regex ou des commandes shell.

## Syntaxe de base

Les règles personnalisées sont définies dans la section `[rules.custom]` du fichier `.repolens.toml` :

```toml
[rules.custom."rule-id"]
pattern = "regex pattern"  # OU
command = "shell command"  # L'un des deux est requis
severity = "warning"        # critical, warning, ou info
files = ["**/*.rs"]        # Optionnel : filtres de fichiers (uniquement pour pattern)
message = "Message personnalisé"
description = "Description détaillée"
remediation = "Comment corriger"
invert = false             # Si true, inverse la logique (pattern non trouvé ou commande échoue)
```

## Règles basées sur des patterns regex

### Exemple simple : Détecter les TODO

```toml
[rules.custom."no-todo"]
pattern = "TODO"
severity = "warning"
files = ["**/*.rs", "**/*.ts"]
message = "TODO comment found"
remediation = "Address or remove the TODO comment"
```

### Exemple avec pattern inversé : Vérifier la présence d'un pattern

```toml
[rules.custom."require-module-doc"]
pattern = "^//!"
severity = "info"
files = ["**/lib.rs"]
invert = true  # Fail si le pattern n'est PAS trouvé
message = "Missing module documentation"
remediation = "Add module-level documentation with //!"
```

### Exemples de patterns courants

```toml
# Détecter les FIXME
[rules.custom."no-fixme"]
pattern = "FIXME"
severity = "critical"
files = ["**/*.rs"]

# Détecter les console.log (pour JavaScript)
[rules.custom."no-console-log"]
pattern = "console\\.log"
severity = "warning"
files = ["**/*.js", "**/*.ts"]

# Vérifier la présence d'un header de licence
[rules.custom."require-license-header"]
pattern = "^// Copyright"
severity = "warning"
files = ["**/*.rs"]
invert = true
```

## Règles basées sur des commandes shell

### Exemple : Vérifier que le répertoire de travail est propre

```toml
[rules.custom."check-git-status"]
command = "git status --porcelain"
severity = "warning"
invert = true  # Fail si la commande retourne un code non-zéro (fichiers modifiés)
message = "Working directory is not clean"
remediation = "Commit or stash your changes"
```

### Exemple : Vérifier qu'un fichier existe

```toml
[rules.custom."check-dockerfile"]
command = "test -f Dockerfile"
severity = "info"
invert = true  # Fail si Dockerfile n'existe pas
message = "Dockerfile not found"
```

### Exemple : Vérifier une version minimale

```toml
[rules.custom."check-rust-version"]
command = "rustc --version | grep -q 'rustc 1\\.7[0-9]'"
severity = "warning"
invert = true
message = "Rust version must be >= 1.70"
```

## Paramètres détaillés

### `pattern` (optionnel si `command` est défini)

- **Type** : `String`
- **Description** : Pattern regex à rechercher dans les fichiers
- **Exemple** : `"TODO"`, `"^//!"`, `"console\\.log"`

### `command` (optionnel si `pattern` est défini)

- **Type** : `String`
- **Description** : Commande shell à exécuter
- **Comportement** : 
  - Si `invert=false` : La règle se déclenche si la commande retourne le code de sortie 0
  - Si `invert=true` : La règle se déclenche si la commande retourne un code non-zéro
- **Exemple** : `"git status --porcelain"`, `"test -f Dockerfile"`

### `severity`

- **Type** : `String`
- **Valeurs possibles** : `"critical"`, `"warning"`, `"info"`
- **Défaut** : `"warning"`
- **Description** : Niveau de sévérité de la règle

### `files` (uniquement pour les règles `pattern`)

- **Type** : `Array<String>`
- **Défaut** : `[]` (tous les fichiers)
- **Description** : Patterns glob pour filtrer les fichiers à vérifier
- **Exemple** : `["**/*.rs"]`, `["src/**", "tests/**"]`

### `message`

- **Type** : `String` (optionnel)
- **Description** : Message personnalisé affiché quand la règle se déclenche
- **Défaut** : Message généré automatiquement

### `description`

- **Type** : `String` (optionnel)
- **Description** : Description détaillée du problème
- **Défaut** : Description générée automatiquement

### `remediation`

- **Type** : `String` (optionnel)
- **Description** : Suggestion de correction
- **Défaut** : Aucune

### `invert`

- **Type** : `Boolean`
- **Défaut** : `false`
- **Description** : Inverse la logique de la règle
  - Pour `pattern` : Se déclenche si le pattern n'est **pas** trouvé
  - Pour `command` : Se déclenche si la commande retourne un code **non-zéro**

## Cas d'usage courants

### Vérifications de qualité de code

```toml
# Pas de debug dans le code de production
[rules.custom."no-debug"]
pattern = "dbg!"
severity = "warning"
files = ["src/**"]

# Pas de unwrap() sans expect()
[rules.custom."no-bare-unwrap"]
pattern = "\\.unwrap\\(\\)"
severity = "critical"
files = ["src/**"]
```

### Vérifications de fichiers requis

```toml
# Vérifier la présence de README
[rules.custom."require-readme"]
command = "test -f README.md"
severity = "warning"
invert = true

# Vérifier la présence de .gitignore
[rules.custom."require-gitignore"]
command = "test -f .gitignore"
severity = "info"
invert = true
```

### Vérifications de configuration

```toml
# Vérifier que les tests passent
[rules.custom."tests-must-pass"]
command = "cargo test --quiet"
severity = "critical"
invert = true

# Vérifier le formatage
[rules.custom."check-formatting"]
command = "cargo fmt --check"
severity = "warning"
invert = true
```

## Bonnes pratiques

1. **Utilisez des IDs descriptifs** : `"no-todo"` plutôt que `"rule1"`
2. **Choisissez la bonne sévérité** : 
   - `critical` : Bloque le merge/CI
   - `warning` : À corriger mais non bloquant
   - `info` : Information utile
3. **Limitez le scope avec `files`** : Évitez de scanner tous les fichiers si possible
4. **Ajoutez des messages clairs** : Aidez les développeurs à comprendre le problème
5. **Testez vos patterns regex** : Utilisez un outil comme [regex101.com](https://regex101.com)
6. **Sécurisez les commandes shell** : Évitez les commandes qui modifient le système

## Limitations

- Les commandes shell sont exécutées dans le répertoire racine du projet
- Les patterns glob supportent `*` et `**` mais pas toutes les fonctionnalités avancées
- Les règles shell ne peuvent pas filtrer par fichiers (le paramètre `files` est ignoré)
- Les commandes shell doivent être disponibles dans le PATH du système

## Exemples complets

Voir le fichier `.repolens.example.toml` pour des exemples complets de configuration.
