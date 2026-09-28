# {{ project_name }}

Build tool: `{{ build_tool }}`. Java version: `{{ java_version }}`.

## Commands

```bash
jav build
jav test
{% if is_runnable %}jav run
{% endif %}```

{% if not is_runnable %}This project has no application entry point. Use its JAR as a dependency or run its tests.
{% endif %}
Native commands:

```bash
{% if is_maven %}mvn package
mvn test
mvn -Prelease package
{% else %}gradle build
gradle test
gradle -Pjav.configuration=release build
{% endif %}```

Debug builds include debug information. Release builds omit it.
{% if is_maven %}Maven stores each configuration in `target/debug` or `target/release`.
{% endif %}

{% if spring_has_postgresql %}
## Local PostgreSQL

Start the dedicated local database before starting the application:

```bash
docker compose up -d --wait
jav run
```

The database uses port 5432, database `{{ project_name }}`, user `app`, and password `local-dev`.
Override `DATABASE_URL`, `DATABASE_USER`, and `DATABASE_PASSWORD` for another database.
The generated JPA configuration updates the schema for local development.
Tests use a separate in-memory H2 database and do not require PostgreSQL.
Stop the database with `docker compose down`.
{% elif spring_has_data_jpa or spring_has_batch %}
## Local database

The application uses an in-memory H2 database. It requires no external database service.
Tests use a separate H2 database.
{% endif %}
{% if spring_has_batch %}
The batch application creates its schema and executes `sampleJob` at startup.
Without the web feature, the process exits after the job completes.
{% endif %}
{% if include_flake %}
## Nix

Initialize the dependency lock with network access before running sandboxed checks:

```bash
git init
git add .
nix run .#lock-deps
git add {% if is_maven %}maven-deps.hash{% else %}deps.json{% endif %} flake.lock
nix develop -c {% if is_maven %}mvn{% else %}gradle{% endif %} test
nix fmt .
nix develop -c prek run --all-files
nix flake check
nix build
```

Run native commands through Nix apps:

```bash
nix run .#build
nix run .#test
{% if is_runnable %}nix run .#run
{% endif %}```

These apps do not require {% if is_maven %}`maven-deps.hash`{% else %}a populated `deps.json`{% endif %}.
They use the native dependency cache and can access the network.
`nix run .` {% if is_runnable %}starts the application{% else %}runs the tests{% endif %}.

The initial project has no resolved dependency lock. Java checks cannot pass until dependency locking completes.
Repeat `nix run .#lock-deps` after changing dependencies, Java, build plugins, or Nixpkgs.
Commit the dependency lock and `flake.lock` together.
Maven records the normalized repository hash. Gradle records downloaded files through the Nixpkgs dependency proxy.
The build and test checks compile Java and execute tests in the same derivation.
Checks use locked dependencies without external network access.
The development shell installs `prek` hooks for Java and Nix formatting.
GitHub Actions runs the same flake checks.
{% endif %}
