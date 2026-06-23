# jav

`jav` is a CLI for Java project creation, detection, build/test/run workflows, and release-binary upgrades.

## Current State

This repository currently contains the first vertical slice:

- Rust CLI project
- `jav doctor`
- `jav new console`
- `jav new library`
- `jav new springboot`
- `jav build`
- `jav test`
- `jav run`
- `jav clean`
- GitHub Releases upgrade checks and binary replacement
- GitHub Actions CI and release automation
- Nix flake for build, check, and release helpers

## Build

```bash
cargo build --release
```

Run locally:

```bash
cargo run -- doctor
cargo run -- new console --name Demo --package dev.example.demo
cargo run -- new springboot --name Api --package dev.example.api --feature web --feature actuator
cargo run -- build
cargo run -- test
cargo run -- upgrade --check
```

With Nix:

```bash
nix develop
nix run . -- doctor
nix run .#check
nix build .#default
nix run .#set-version
nix run .#set-version -- 2026.06.23.1
```

`Cargo.toml` is the source of truth for the package and release version.

Templates currently support:

- `console`
- `library`
- `springboot`

Template options currently support:

- `--build-tool maven|gradle`
- `--feature ...` for `springboot`
- `--spring-boot-version` for `springboot`

Install locally on Windows:

```powershell
.\scripts\install.ps1
```

Install from the latest GitHub release:

```powershell
irm https://raw.githubusercontent.com/sachahjkl/jav/master/scripts/install.ps1 | iex
```

## Install

### Nix

Run the CLI without installing it:

```bash
nix run github:sachahjkl/jav -- doctor
```

Refresh to the latest pushed revision when needed:

```bash
nix run --refresh github:sachahjkl/jav -- --version
```

Install it into your Nix profile for repeated use:

```bash
nix profile install github:sachahjkl/jav
jav --version
```

Upgrade a profile install:

```bash
nix profile upgrade github:sachahjkl/jav
```

`jav upgrade` is disabled for Nix-managed installs. Use `nix run --refresh ...` or `nix profile upgrade ...` instead.

### Release Binaries

Windows install from the latest GitHub release:

```powershell
irm https://raw.githubusercontent.com/sachahjkl/jav/master/scripts/install.ps1 | iex
```

Default install location:

```text
%LOCALAPPDATA%\jav\bin
```

The installer adds this directory to the user `PATH` unless `-NoPathUpdate` is passed.

Linux/WSL install from the latest GitHub release:

```bash
curl -fsSL https://raw.githubusercontent.com/sachahjkl/jav/master/scripts/install.sh | sh
```

Default install location:

```text
~/.local/bin
```

The Linux/WSL installer detects the current shell and updates the matching init file when possible:

- bash: `~/.bashrc`
- zsh: `~/.zshrc`
- fish: `~/.config/fish/config.fish`
- nushell: `~/.config/nushell/env.nu`
- PowerShell: `~/.config/powershell/Microsoft.PowerShell_profile.ps1`

Use `JAV_NO_PATH_UPDATE=1` or `--no-path-update` to skip shell profile changes.

## CI and Release

CI runs on pull requests and pushes to `develop`, `main`, or `master`:

- Windows job: build, test, package `win-x64`
- Linux job: install Nix, run `nix build .#default`, run `nix run .#check`, publish `linux-x64`

Releases are automated from `master`.

Before pushing a release commit, bump `Cargo.toml`:

```bash
git fetch --tags
nix run .#set-version
git add Cargo.toml Cargo.lock
git commit -m "bump version"
git push origin master
```

The release workflow reads `Cargo.toml` and fails early if `v<version>` already exists.

When a commit lands on `master`, `.github/workflows/release.yml`:

1. reads the version and creates the matching tag
2. publishes Windows and Linux artifacts
3. creates a GitHub Release
4. uploads:
   - `jav-win-x64.zip`
   - `jav-linux-x64.tar.gz`
   - `release.json`

`release.json` is the manifest used by `jav upgrade --check` and `jav upgrade`.
