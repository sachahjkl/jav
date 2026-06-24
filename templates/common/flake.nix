{
  description = "{{ project_name }}";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      forAllSystems = nixpkgs.lib.genAttrs systems;

      projectFor = system:
        let
          pkgs = import nixpkgs { inherit system; };
          buildTool = {% if is_maven %}pkgs.maven{% else %}pkgs.gradle{% endif %};
        in
        {
          devShells.default = pkgs.mkShell {
            packages = [
              buildTool
              pkgs.jdk{{ java_version }}
            ];

            JAVA_HOME = pkgs.jdk{{ java_version }};
          };

          apps = {
            build = {
              type = "app";
              program = "${pkgs.writeShellScript "{{ project_name }}-build" ''
                {% if is_maven %}exec ${buildTool}/bin/mvn package "$@"{% else %}exec ${buildTool}/bin/gradle build "$@"{% endif %}
              ''}";
            };

            test = {
              type = "app";
              program = "${pkgs.writeShellScript "{{ project_name }}-test" ''
                {% if is_maven %}exec ${buildTool}/bin/mvn test "$@"{% else %}exec ${buildTool}/bin/gradle test "$@"{% endif %}
              ''}";
            };

            run = {
              type = "app";
              program = "${pkgs.writeShellScript "{{ project_name }}-run" ''
                {% if is_maven and is_spring %}exec ${buildTool}/bin/mvn spring-boot:run "$@"{% elif is_maven %}exec ${buildTool}/bin/mvn compile exec:java "$@"{% elif is_spring %}exec ${buildTool}/bin/gradle bootRun "$@"{% else %}exec ${buildTool}/bin/gradle run "$@"{% endif %}
              ''}";
            };

            default = self.apps.${system}.run;
          };
        };
    in
    {
      devShells = forAllSystems (system: (projectFor system).devShells);
      apps = forAllSystems (system: (projectFor system).apps);
    };
}
