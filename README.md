# jav

`jav` is a dotnet-inspired CLI for Java projects.

It gives Java a single, friendly command for creating, building, running, testing, and inspecting projects while integrating with Maven, Gradle, and the broader Java ecosystem.

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
