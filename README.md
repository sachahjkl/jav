<div align="center">

<img src=".project/image.png" alt="jav logo" width="192" height="192">

# jav

**One command line for everyday Java development.**

[![Latest release](https://img.shields.io/github/v/release/sachahjkl/jav?style=for-the-badge&color=ef4444)](https://github.com/sachahjkl/jav/releases/latest)
[![CI](https://img.shields.io/github/actions/workflow/status/sachahjkl/jav/ci.yml?branch=master&style=for-the-badge&label=CI&color=2563eb)](https://github.com/sachahjkl/jav/actions/workflows/ci.yml)
[![Nix](https://img.shields.io/badge/Nix-ready-5277C3?style=for-the-badge&logo=nixos&logoColor=white)](https://nixos.org/)
[![Java 21](https://img.shields.io/badge/Java-21-f97316?style=for-the-badge&logo=openjdk&logoColor=white)](https://openjdk.org/projects/jdk/21/)

[Install](#install) · [Quick start](#quick-start) · [Templates](#templates) · [Commands](#commands) · [Demo](#demo)

</div>

---

`jav` gives Maven, Gradle, and simple Java projects one consistent workflow.
Create a project, build it, test it, and run it without switching command styles.

It does not replace your build tool. It detects the project and calls the native
tool with predictable defaults.

```console
$ jav build
$ jav test
$ jav run -- hello world
```

## Why jav?

| | |
|---|---|
| **One workflow** | Use the same commands across Maven, Gradle, and simple Java layouts. |
| **Useful templates** | Start console, CLI, library, worker, JUnit, and Spring projects. |
| **Project native** | Keep normal `pom.xml`, Gradle files, source layouts, and build outputs. |
| **Reproducible by default** | Generate a Nix flake, `prek` hooks, and GitHub Actions with new projects. |
| **Fast feedback** | Build only when sources or project inputs are newer than existing outputs. |
| **Self-contained releases** | Install native Linux or Windows binaries and upgrade them with `jav upgrade`. |

## Demo

Create and run a Java console application in three commands.

<p align="center">
  <a href="docs/demo.cast">
    <img src="docs/demo.gif" alt="Terminal demo: create and run a Java project with jav" width="910">
  </a>
</p>

The animation comes from the committed [asciinema recording](docs/demo.cast).

## Quick start

```sh
jav new console --name HelloJav --package dev.example.hello
cd HelloJav
jav run
```

`jav run` builds first when project inputs are newer than the output. Pass
`--no-build` when you explicitly want to skip that check.

Explore the installed templates:

```sh
jav new list --verbose
jav new springweb --describe
```

## Install

### Nix

Run without installing:

```sh
nix run github:sachahjkl/jav -- doctor
```

Install into your profile:

```sh
nix profile install github:sachahjkl/jav
```

### Linux

```sh
curl -fsSL https://raw.githubusercontent.com/sachahjkl/jav/master/scripts/install.sh | sh
```

### Windows PowerShell

```powershell
irm https://raw.githubusercontent.com/sachahjkl/jav/master/scripts/install.ps1 | iex
```

Release-binary installations can update themselves:

```sh
jav upgrade --check
jav upgrade
```

Use Nix to update installations managed by Nix.

## Commands

| Command | Purpose |
|---|---|
| `jav new` | Create a project from an installed template. |
| `jav build` | Build the current project. |
| `jav test` | Run the current project's tests. |
| `jav run` | Build when necessary, then run the project. |
| `jav clean` | Remove native build outputs. |
| `jav doctor` | Inspect the available Java development tools. |
| `jav upgrade` | Update a release-binary installation. |

Common workflows:

```sh
jav build --configuration release
jav run --configuration debug -- hello world
jav run --no-build -- server --port 8080
jav clean
```

Build and run support `debug` and `release` configurations. Generated Maven
projects use profiles. Generated Gradle projects use the corresponding
`jav.configuration` property.

## Templates

| Template | Default tool | Purpose |
|---|---|---|
| `console` | Maven | Executable Java application with JUnit tests. |
| `cli` | Gradle | Command-line application skeleton. |
| `worker` | Gradle | Long-running background worker. |
| `library` | Maven | Reusable Java library. |
| `junit` | Maven | Focused JUnit 5 test project. |
| `springboot` | Gradle | Configurable Spring Boot application. |
| `springweb` | Gradle | Spring REST API. |
| `springdata` | Gradle | Spring API with JPA and PostgreSQL scaffolding. |
| `springsecurity` | Gradle | Spring API with security defaults. |
| `springbatch` | Gradle | Spring Batch job starter. |

Use `--build-tool maven` or `--build-tool gradle` to override a template's
default.

```sh
jav new library --name Core --build-tool gradle
jav new springboot --name Service --feature web --feature actuator
jav new springdata --name Api --spring-boot-version 3.5.0
```

Template aliases include `webapi` for `springweb` and `classlib` for `library`.

## Project configuration

Generated runnable projects include `jav.toml`:

```toml
[run]
main_class = "dev.example.hello.Application"
maven_task = "exec:java"
gradle_task = "run"
```

Edit these values when a project needs a custom main class or run task.

New projects also include a Nix flake and a GitHub Actions workflow by default.
Pass `--no-flake` when you do not want the Nix files.

## Development

Enter the development environment:

```sh
nix develop
```

Run the checks:

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
nix flake check "path:$PWD" --no-write-lock-file
```

`Cargo.toml` is the source of truth for the package version. Releases are built
from `master` for Linux and Windows.

## Contributing

Bug reports and focused pull requests are welcome. Include a minimal project,
the command you ran, and the complete error when reporting a problem.

- [Open an issue](https://github.com/sachahjkl/jav/issues/new)
- [Browse releases](https://github.com/sachahjkl/jav/releases)
