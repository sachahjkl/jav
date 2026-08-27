{
  description = "{{ project_name }}";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1";
    git-hooks = {
      url = "https://flakehub.com/f/cachix/git-hooks.nix/0.1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    git-hooks,
    ...
  }: let
    systems = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];

    forAllSystems = nixpkgs.lib.genAttrs systems;

    projectFor = system: let
      pkgs = import nixpkgs {inherit system;};
      buildTool = {% if is_maven %}pkgs.maven{% else %}pkgs.gradle{% endif %};
      jdk = pkgs.jdk{{ java_version }};
      mkCommand = name: command:
        pkgs.writeShellApplication {
          inherit name;
          runtimeInputs = [buildTool jdk];
          runtimeEnv.JAVA_HOME = jdk;
          text = ''
            exec ${command} "$@"
          '';
        };
      buildScript = mkCommand "{{ project_name }}-build" "{% if is_maven %}mvn package{% else %}gradle build{% endif %}";
      testScript = mkCommand "{{ project_name }}-test" "{% if is_maven %}mvn test{% else %}gradle test{% endif %}";
      runScript = mkCommand "{{ project_name }}-run" "{% if is_maven and is_spring %}mvn spring-boot:run{% elif is_maven %}mvn compile exec:java{% elif is_spring %}gradle bootRun{% else %}gradle run{% endif %}";
      preCommitCheck = git-hooks.lib.${system}.run {
        package = pkgs.prek;
        src = ./.;
        hooks = {
          check-added-large-files.enable = true;
          check-merge-conflicts.enable = true;
          alejandra.enable = true;
          end-of-file-fixer.enable = true;
          google-java-format.enable = true;
          trim-trailing-whitespace.enable = true;
        };
      };
    in {
      devShells.default = pkgs.mkShell {
        packages = preCommitCheck.enabledPackages ++ [buildTool jdk];

        JAVA_HOME = jdk;
        inherit (preCommitCheck) shellHook;
      };

      apps = {
        build = {
          type = "app";
          program = "${buildScript}/bin/{{ project_name }}-build";
        };

        test = {
          type = "app";
          program = "${testScript}/bin/{{ project_name }}-test";
        };

        run = {
          type = "app";
          program = "${runScript}/bin/{{ project_name }}-run";
        };

        default = self.apps.${system}.run;
      };

      checks = {
        build-script = buildScript;
        pre-commit = preCommitCheck;
        test-script = testScript;
      };

      formatter = pkgs.alejandra;
    };
  in {
    devShells = forAllSystems (system: (projectFor system).devShells);
    apps = forAllSystems (system: (projectFor system).apps);
    checks = forAllSystems (system: (projectFor system).checks);
    formatter = forAllSystems (system: (projectFor system).formatter);
  };
}
