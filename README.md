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
```

## Development

```sh
nix develop
cargo test
cargo run -- doctor
```

## Nix

```sh
nix run . -- doctor
nix build
```
