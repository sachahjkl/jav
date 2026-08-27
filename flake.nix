{
  description = "jav - a modern CLI for Java projects";

  nixConfig = {
    extra-substituters = [
      "https://sachahjkl.cachix.org"
    ];
    extra-trusted-public-keys = [
      "sachahjkl.cachix.org-1:cepX7PCUV88hCchnh9prZM5V72wRkCf6oSJL6JfgWs0="
    ];
  };

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1";
    flake-utils.url = "github:numtide/flake-utils";
    git-hooks = {
      url = "https://flakehub.com/f/cachix/git-hooks.nix/0.1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    git-hooks,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {inherit system;};
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        version = cargoToml.package.version;
        src = pkgs.lib.cleanSource ./.;
        sourceRevision =
          self.shortRev
            or (
            if self ? rev
            then builtins.substring 0 7 self.rev
            else self.dirtyShortRev or "dev"
          );

        buildScript = pkgs.writeShellApplication {
          name = "jav-build";
          runtimeInputs = with pkgs; [
            cargo
            rustc
          ];
          text = ''
            cargo build --release --locked
          '';
        };

        checkScript = pkgs.writeShellApplication {
          name = "jav-check";
          runtimeInputs = with pkgs; [
            cargo
            rustc
          ];
          text = ''
            cargo test --locked
            cargo run --locked -- doctor
          '';
        };

        publishLinuxX64Script = pkgs.writeShellApplication {
          name = "jav-publish-linux-x64";
          runtimeInputs = with pkgs; [
            bash
            cargo
            rustc
            coreutils
            gawk
            perl
          ];
          text = ''
            COMMIT=${sourceRevision} bash ./scripts/publish-linux-x64.sh
          '';
        };

        setVersionScript = pkgs.writeShellApplication {
          name = "jav-set-version";
          runtimeInputs = with pkgs; [
            bash
            coreutils
            git
            gnused
            perl
          ];
          text = ''
            bash ./scripts/set-version.sh "$@"
          '';
        };

        package = pkgs.rustPlatform.buildRustPackage {
          pname = "jav";
          inherit version src;
          cargoLock.lockFile = ./Cargo.lock;
        };

        clippyCheck = pkgs.rustPlatform.buildRustPackage {
          pname = "jav-clippy";
          inherit version src;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [pkgs.clippy];
          buildPhase = ''
            cargo clippy --all-targets -- -D warnings
          '';
          installPhase = "touch $out";
          doCheck = false;
        };

        formatCheck =
          pkgs.runCommand "jav-format"
          {
            inherit src;
            nativeBuildInputs = [
              pkgs.cargo
              pkgs.rustfmt
            ];
          }
          ''
            cp -r "$src" source
            chmod -R +w source
            cd source
            cargo fmt --all --check
            touch "$out"
          '';

        preCommitCheck = git-hooks.lib.${system}.run {
          package = pkgs.prek;
          src = ./.;
          hooks = {
            check-added-large-files.enable = true;
            check-merge-conflicts.enable = true;
            alejandra = {
              enable = true;
              excludes = ["^templates/common/flake\\.nix$"];
            };
            deadnix = {
              enable = true;
              excludes = ["^templates/common/flake\\.nix$"];
            };
            end-of-file-fixer.enable = true;
            rustfmt.enable = true;
            statix = {
              enable = true;
              settings.ignore = ["templates/common/flake.nix"];
            };
            trim-trailing-whitespace.enable = true;
          };
        };
      in {
        packages.default = package;

        checks = {
          build = package;
          clippy = clippyCheck;
          format = formatCheck;
          pre-commit = preCommitCheck;
        };

        apps = {
          build = {
            type = "app";
            program = "${buildScript}/bin/jav-build";
          };

          check = {
            type = "app";
            program = "${checkScript}/bin/jav-check";
          };

          publish-linux-x64 = {
            type = "app";
            program = "${publishLinuxX64Script}/bin/jav-publish-linux-x64";
          };

          set-version = {
            type = "app";
            program = "${setVersionScript}/bin/jav-set-version";
          };

          jav = flake-utils.lib.mkApp {
            drv = self.packages.${system}.default;
          };

          default = self.apps.${system}.jav;
        };

        devShells.default = pkgs.mkShell {
          packages =
            preCommitCheck.enabledPackages
            ++ (with pkgs; [
              bash
              cargo
              cargo-nextest
              clippy
              gawk
              gradle
              jdk21_headless
              just
              maven
              perl
              rustc
              rustfmt
            ]);

          JAVA_HOME = pkgs.jdk21_headless;

          shellHook = ''
            ${preCommitCheck.shellHook}
            echo "jav dev shell"
            echo "Commands:"
            echo "  nix run .#build"
            echo "  nix run .#check"
            echo "  nix run .#publish-linux-x64"
            echo "  nix run .#set-version"
            echo "  nix run .#set-version -- 2026.623.1"
            echo ""
            echo "Version: ${version}+${sourceRevision}"
          '';
        };

        formatter = pkgs.alejandra;
      }
    );
}
