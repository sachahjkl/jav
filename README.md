# jav

`jav` is a modern CLI for Java projects.

It provides a single, reliable command for creating, building, running, testing, and inspecting Java projects while integrating with Maven, Gradle, and the broader Java ecosystem.

## MVP commands

```sh
jav doctor
jav new console -n Demo
jav new library -n Shared
jav build
jav test
jav run
jav clean
jav upgrade --check
```

## Development

```sh
nix develop
cargo test
cargo run -- doctor
cargo run -- upgrade --check
```

## Nix

```sh
nix run . -- doctor
nix build
nix run .#check
```

## Release automation

```sh
nix run .#set-version
nix run .#set-version -- 2026.06.23.1
```

`Cargo.toml` is the release version source, and `scripts/set-version.sh` updates it.

`jav upgrade` reads the latest GitHub release manifest (`release.json`), verifies the asset SHA256, and replaces the current binary for non-Nix installs. Configure the release source with:

```sh
export JAV_UPGRADE_OWNER=your-github-owner
export JAV_UPGRADE_REPOSITORY=jav
```
