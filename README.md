# jav

`jav` is a CLI for day-to-day Java project work.

It gives you one command surface for:
- creating new Java projects
- detecting the current project type
- building, testing, running, and cleaning projects
- checking local Java tooling
- self-upgrading from GitHub releases

The current implementation targets three project shapes:
- Maven projects
- Gradle projects
- simple source-tree projects with `src/main/java`

## Commands

```sh
jav doctor
jav new console --name Demo --package dev.example.demo
jav new library --name Shared --package dev.example.shared
jav build
jav test
jav run
jav clean
jav upgrade --check
jav upgrade
```

## What each command does

`jav doctor`
- prints the current `jav` version
- checks for `java`, `javac`, `mvn`, and `gradle` on `PATH`
- reports `JAVA_HOME`
- detects whether the current directory looks like a Java project

`jav new console`
- creates a console app template
- writes a Maven project layout
- generates `pom.xml`
- generates `src/main/java/.../Main.java`
- generates `src/test/java/.../MainTest.java`

`jav new library`
- creates a library template
- writes a Maven project layout
- generates `pom.xml`
- generates `src/main/java/.../Library.java`
- generates `src/test/java/.../LibraryTest.java`

`jav build`
- runs `mvn package` in Maven projects
- runs `gradle build` in Gradle projects
- compiles `src/main/java/**/*.java` into `out/` for simple projects

`jav test`
- runs `mvn test` in Maven projects
- runs `gradle test` in Gradle projects
- currently errors for simple projects

`jav run`
- runs `mvn exec:java` in Maven projects
- runs `gradle run` in Gradle projects
- currently errors for simple projects

`jav clean`
- runs `mvn clean` in Maven projects
- runs `gradle clean` in Gradle projects
- removes `out/` for simple projects

`jav upgrade --check`
- queries the latest GitHub release
- downloads `release.json`
- prints the current version and available release assets

`jav upgrade`
- finds the asset matching the current runtime identifier
- downloads the release asset
- verifies the SHA256 from `release.json`
- replaces the current executable
- refuses to run for Nix-managed installs

## Project detection

`jav` chooses behavior from the current directory:
- `pom.xml` => Maven
- `build.gradle`, `build.gradle.kts`, `settings.gradle`, or `settings.gradle.kts` => Gradle
- `src/main/java` => simple project

If none of those are present, commands that need a project fail with:
- `not in a Java project; expected pom.xml, build.gradle, or src/main/java`

## New project generation

Supported templates:
- `console`
- `library`

Arguments:
- `--name` project name
- `--package` Java package name
- `--output` output directory, defaults to the project name
- `--java-version` Java language version, defaults to `21`

Validation rules:
- project name cannot be empty
- project name cannot contain path separators
- package names must be dotted identifiers
- each package segment must start with a letter or `_`
- package segments may only contain letters, digits, and `_`

If you omit `--package`, `jav` generates a default package like:

```sh
com.example.demo
```

## Installation

### Nix

Run directly from the flake:

```sh
nix run . -- doctor
```

Build the package:

```sh
nix build
```

Run the repo validation app:

```sh
nix run .#check
```

Enter the development shell:

```sh
nix develop
```

### Cargo

For local development:

```sh
cargo run -- doctor
cargo test
```

Install from the current checkout:

```sh
cargo install --path .
```

## Upgrade behavior

By default, `jav upgrade` reads releases from:

```text
https://github.com/sachahjkl/jav
```

It expects a GitHub release asset named `release.json` with a manifest like:
- version
- commit
- per-platform assets
- SHA256 checksums

Optional overrides:

```sh
export JAV_UPGRADE_OWNER=sachahjkl
export JAV_UPGRADE_REPOSITORY=jav
export JAV_UPGRADE_PRERELEASE=false
export JAV_UPGRADE_ASSET=release.json
```

Notes:
- automatic upgrade currently supports `linux-x64` and `win-x64`
- Nix-managed binaries are intentionally not replaced in place
- on Linux, the binary is replaced directly
- on Windows, replacement is delegated to a temporary `cmd` script after process exit

## Release automation

The release version comes from `Cargo.toml`.

To set a version manually:

```sh
nix run .#set-version -- 2026.06.23.1
```

To generate the next date-based version automatically:

```sh
nix run .#set-version
```

That command:
- uses the current UTC date as `YYYY.MM.DD`
- counts existing tags matching `vYYYY.MM.DD.*`
- writes the next version back to `Cargo.toml`

GitHub Actions includes:
- CI workflow for Linux and Windows
- release workflow that tags `v<version>`
- packaged release assets for `linux-x64` and `win-x64`
- generated `release.json` for `jav upgrade`

## Development

Useful commands:

```sh
nix develop
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -- doctor
cargo run -- upgrade --check
```

There is also a `justfile` with:
- `just fmt`
- `just lint`
- `just test`
- `just nextest`
- `just check`

## Current limitations

- `new` currently generates Maven-based templates only
- simple projects support `build` and `clean`, but not `run` or `test`
- Maven `run` assumes `mvn exec:java` works for the project
- upgrade support is implemented for Linux and Windows x64 only
