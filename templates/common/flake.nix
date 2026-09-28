{
  description = "{{ project_name }}";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.2605";
    git-hooks = {
      url = "https://flakehub.com/f/cachix/git-hooks.nix/0.1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {nixpkgs, git-hooks, ...}: let
    systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
    forAllSystems = nixpkgs.lib.genAttrs systems;
    projectFor = system: let
      pkgs = import nixpkgs {inherit system;};
      jdk = pkgs.jdk{{ java_version }};
      buildTool = {% if is_maven %}pkgs.maven{% else %}pkgs.gradle.override {java = jdk;}{% endif %};
      mkCommand = name: command: pkgs.writeShellApplication {
        inherit name;
        runtimeInputs = [buildTool jdk];
        runtimeEnv.JAVA_HOME = jdk;
        text = ''
          exec ${command} "$@"
        '';
      };
      buildScript = mkCommand "project-build" "{% if is_maven %}mvn package{% else %}gradle build{% endif %}";
      testScript = mkCommand "project-test" "{% if is_maven %}mvn test{% else %}gradle test{% endif %}";
      {% if is_runnable %}
      runScript = mkCommand "project-run" "{% if is_maven and is_spring %}mvn spring-boot:run{% elif is_maven %}mvn compile exec:java{% elif is_spring %}gradle bootRun{% else %}gradle run{% endif %}";
      {% endif %}
      buildApp = {type = "app"; program = "${buildScript}/bin/project-build";};
      testApp = {type = "app"; program = "${testScript}/bin/project-test";};
      {% if is_runnable %}
      runApp = {type = "app"; program = "${runScript}/bin/project-run";};
      {% endif %}
      src = pkgs.lib.cleanSourceWith {
        src = ./.;
        filter = path: type: let
          parts = pkgs.lib.splitString "/" (pkgs.lib.removePrefix "${toString ./.}/" (toString path));
        in
          pkgs.lib.cleanSourceFilter path type
          && (builtins.elem "src" parts || !(builtins.elem (baseNameOf path) ["target" "build" ".gradle" ".m2" "result"]));
      };
      {% if is_maven %}
      package = buildTool.buildMavenPackage {
        pname = "{{ project_name }}";
        version = "0.1.0";
        inherit src;
        mvnJdk = jdk;
        mvnHash = if builtins.pathExists ./maven-deps.hash
          then pkgs.lib.trim (builtins.readFile ./maven-deps.hash)
          else throw "Run nix run .#lock-deps, then add maven-deps.hash to Git.";
        doCheck = true;
        installPhase = ''
          mkdir -p "$out"
          cp target/debug/*.jar "$out/"
        '';
      };
      lockScript = pkgs.writeShellApplication {
        name = "lock-deps";
        runtimeInputs = [buildTool jdk pkgs.nix pkgs.coreutils pkgs.findutils];
        runtimeEnv.JAVA_HOME = jdk;
        text = ''
          temporary=$(mktemp -d)
          trap 'rm -rf "$temporary"' EXIT
          mkdir -p "$temporary/dependencies/.m2"
          mvn -B package -Dmaven.repo.local="$temporary/dependencies/.m2"
          find "$temporary/dependencies" -type f \( -name '*.lastUpdated' -o -name resolver-status.properties -o -name _remote.repositories \) -delete
          rm -f "$temporary/dependencies"/.m2/.meta/prefixes-*.txt{,.*}
          nix hash path "$temporary/dependencies" > maven-deps.hash
        '';
      };
      {% else %}
      package = pkgs.stdenv.mkDerivation (finalAttrs: {
        pname = "{{ project_name }}";
        version = "0.1.0";
        inherit src;
        nativeBuildInputs = [buildTool];
        mitmCache = buildTool.fetchDeps {
          pkg = finalAttrs.finalPackage;
          data = ./deps.json;
        };
        __darwinAllowLocalNetworking = true;
        gradleFlags = ["-Dorg.gradle.java.home=${jdk}"];
        gradleBuildTask = "assemble";
        gradleCheckTask = "test";
        gradleUpdateTask = "build";
        doCheck = true;
        installPhase = ''
          mkdir -p "$out"
          cp build/libs/*.jar "$out/"
        '';
      });
      lockScript = pkgs.writeShellApplication {
        name = "lock-deps";
        text = ''
          exec ${package.mitmCache.updateScript} "$@"
        '';
      };
      {% endif %}
      preCommitCheck = git-hooks.lib.${system}.run {
        package = pkgs.prek;
        inherit src;
        hooks = {
          alejandra.enable = true;
          deadnix.enable = true;
          statix.enable = true;
          google-java-format.enable = true;
          check-merge-conflicts.enable = true;
          check-json.enable = true;
          check-toml.enable = true;
          check-yaml.enable = true;
          end-of-file-fixer.enable = true;
          trim-trailing-whitespace.enable = true;
        };
      };
    in {
      packages.default = package;
      checks = {
        build = package;
        test = package;
        pre-commit = preCommitCheck;
      };
      apps = {
        build = buildApp;
        test = testApp;
        {% if is_runnable %}run = runApp;{% endif %}
        default = {% if is_runnable %}runApp{% else %}testApp{% endif %};
        lock-deps = {
          type = "app";
          program = "${lockScript}/bin/lock-deps";
        };
      };
      devShells.default = pkgs.mkShell {
        packages = preCommitCheck.enabledPackages ++ [buildTool jdk pkgs.prek];
        JAVA_HOME = jdk;
        inherit (preCommitCheck) shellHook;
      };
      formatter = pkgs.alejandra;
    };
  in {
    packages = forAllSystems (system: (projectFor system).packages);
    checks = forAllSystems (system: (projectFor system).checks);
    apps = forAllSystems (system: (projectFor system).apps);
    devShells = forAllSystems (system: (projectFor system).devShells);
    formatter = forAllSystems (system: (projectFor system).formatter);
  };
}
