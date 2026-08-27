[English](README.md) | [Français](README.fr.md)

# jav

`jav` est une CLI moderne pour les projets Java. Elle crée des projets à partir de modèles et détecte les structures Maven, Gradle et Java simple. Elle fournit une forme de commande unique pour `new`, `build`, `test`, `run`, `clean`, `doctor` et `upgrade`.

Le but n'est pas de remplacer Maven ou Gradle. Le but est d'unifier le flux de travail Java quotidien.

## Démarrage rapide

```bash
jav new
jav new list --verbose
jav new console --name Demo --package dev.example.demo
cd Demo
jav run
```

`jav run` compile d'abord le projet quand les sources sont plus récentes que les résultats. Utilisez `--no-build` pour ignorer explicitement cette vérification.

## Commandes

```bash
jav doctor
jav new list
jav new springweb --describe
jav new springweb --name Api --build-tool gradle
jav build --configuration release
jav test
jav run --configuration debug -- hello world
jav clean
jav upgrade --check
```

La compilation et l'exécution prennent en charge les configurations `debug` et `release`. Pour les projets générés, elles correspondent aux profils Maven ou aux propriétés Gradle natifs de Java.

Les projets exécutables générés contiennent un fichier `jav.toml`. Ce fichier définit les valeurs d'exécution par défaut, comme la classe principale et la tâche Maven ou Gradle. Modifiez-le si un projet nécessite une commande d'exécution personnalisée.

Par défaut, les projets génèrent aussi une `flake.nix` et un flux de travail GitHub Actions. La flake fournit des outils épinglés, des hooks `prek`, des contrôles de formatage et des applications de compilation, de test et d'exécution. Utilisez `--no-flake` si vous ne voulez pas ces fichiers.

## Modèles

Modèles installés :

- `console` : application Java exécutable avec des tests JUnit, utilise Maven par défaut
- `cli` : squelette d'application en ligne de commande, utilise Gradle par défaut
- `worker` : squelette de processus de longue durée ou d'arrière-plan, utilise Gradle par défaut
- `library` : bibliothèque Java réutilisable, utilise Maven par défaut
- `junit` : projet de test JUnit 5 ciblé, utilise Maven par défaut
- `springboot` : application Spring Boot configurable, utilise Gradle par défaut
- `springweb` : API REST Spring, utilise Gradle par défaut
- `springdata` : API Spring avec une structure JPA/PostgreSQL, utilise Gradle par défaut
- `springsecurity` : API Spring avec une configuration de sécurité par défaut, utilise Gradle par défaut
- `springbatch` : projet initial de tâche Spring Batch, utilise Gradle par défaut

Options communes des modèles :

```bash
jav new console --name Demo --package dev.example.demo
jav new library --name Core --build-tool gradle
jav new springboot --name Service --feature web --feature actuator
jav new springdata --name Api --spring-boot-version 3.5.0
```

Les outils de compilation pris en charge sont `maven` et `gradle`. Passez `--build-tool` pour remplacer l'outil par défaut du modèle.

Les alias de modèles sont aussi pris en charge. Par exemple, utilisez `webapi` pour `springweb` et `classlib` pour `library`.

Les versions des dépendances générées sont épinglées dans les fichiers Maven ou Gradle. Pour des compilations entièrement reproductibles, validez `flake.lock`. Activez aussi le verrouillage des dépendances Maven ou Gradle selon les besoins du projet.

## Installation

Exécutez l'application avec Nix sans l'installer :

```bash
nix run github:sachahjkl/jav -- doctor
```

Installez l'application avec Nix :

```bash
nix profile install github:sachahjkl/jav
```

Installez l'application depuis les fichiers binaires d'une version :

```powershell
irm https://raw.githubusercontent.com/sachahjkl/jav/master/scripts/install.ps1 | iex
```

```bash
curl -fsSL https://raw.githubusercontent.com/sachahjkl/jav/master/scripts/install.sh | sh
```

Mettez à niveau les installations gérées par Nix avec Nix. Les installations par fichiers binaires peuvent utiliser `jav upgrade`.

## Développement

```bash
nix develop
cargo run -- new list
cargo run -- build --configuration release
```

Vérifiez les modifications :

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

`Cargo.toml` est la source de référence pour la version du paquet.

## Publication

Les publications sont automatisées depuis `master`. Le flux de travail compile les artefacts Windows et Linux. Il crée la publication GitHub et publie `release.json` pour `jav upgrade`.

Avant un commit de publication :

```bash
nix run .#set-version
git add Cargo.toml Cargo.lock
git commit -m "bump version"
git push origin master
```
