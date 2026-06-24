# jav

`jav` is a modern CLI for Java projects. It creates projects from templates, detects Maven/Gradle/simple Java layouts, and gives you one command shape for `new`, `build`, `test`, `run`, `clean`, `doctor`, and `upgrade`.

The goal is not to replace Maven or Gradle. The goal is to make the everyday Java workflow feel integrated.

## Quick Start

```bash
jav new
jav new list --verbose
jav new console --name Demo --package dev.example.demo
cd Demo
jav run
```

`jav run` builds first when sources are newer than outputs. Use `--no-build` when you explicitly want to skip that check.

## Commands

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

Build and run support `debug` and `release` configurations. For generated projects this maps to Java-native Maven profiles or Gradle properties.

Generated runnable projects include a `jav.toml` file for run defaults such as main class and Maven/Gradle task. Edit it when a project needs a custom run shape.

Projects also generate a `flake.nix` by default for a pinned Java/build-tool environment. Use `--no-flake` if you do not want Nix files.

## Templates

Installed templates:

- `console`: executable Java app with JUnit tests, defaults to Maven
- `cli`: command-line app skeleton, defaults to Gradle
- `worker`: long-running/background worker skeleton, defaults to Gradle
- `library`: reusable Java library, defaults to Maven
- `junit`: focused JUnit 5 test project, defaults to Maven
- `springboot`: configurable Spring Boot app, defaults to Gradle
- `springweb`: Spring REST API, defaults to Gradle
- `springdata`: Spring API with JPA/PostgreSQL scaffolding, defaults to Gradle
- `springsecurity`: Spring API with security defaults, defaults to Gradle
- `springbatch`: Spring Batch job starter, defaults to Gradle

Common template options:

```bash
jav new console --name Demo --package dev.example.demo
jav new library --name Core --build-tool gradle
jav new springboot --name Service --feature web --feature actuator
jav new springdata --name Api --spring-boot-version 3.5.0
```

Supported build tools are `maven` and `gradle`. Pass `--build-tool` to override the template default.

Template aliases are supported too, for example `webapi` for `springweb` and `classlib` for `library`.

Generated dependency versions are pinned in Maven/Gradle files. For fully reproducible project builds, commit `flake.lock` and enable Maven/Gradle dependency locking as needed for the project.

## Install

Run without installing through Nix:

```bash
nix run github:sachahjkl/jav -- doctor
```

Install with Nix:

```bash
nix profile install github:sachahjkl/jav
```

Install from release binaries:

```powershell
irm https://raw.githubusercontent.com/sachahjkl/jav/master/scripts/install.ps1 | iex
```

```bash
curl -fsSL https://raw.githubusercontent.com/sachahjkl/jav/master/scripts/install.sh | sh
```

Nix-managed installs should be upgraded with Nix. Release-binary installs can use `jav upgrade`.

## Development

```bash
nix develop
cargo run -- new list
cargo run -- build --configuration release
```

Verify changes:

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

`Cargo.toml` is the source of truth for the package version.

## Release

Releases are automated from `master`. The release workflow builds Windows and Linux artifacts, creates the GitHub release, and publishes `release.json` for `jav upgrade`.

Before a release commit:

```bash
nix run .#set-version
git add Cargo.toml Cargo.lock
git commit -m "bump version"
git push origin master
```
