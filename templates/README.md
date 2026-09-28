# Template checks

Run the generation tests without network access:

```bash
nix develop --no-write-lock-file -c cargo test --locked --test templates
```

Run the Java integration tests with access to Maven Central and the Gradle plugin portal:

```bash
nix develop --no-write-lock-file -c cargo test --locked --test templates -- --ignored --test-threads=1
```

The integration tests require Maven, Gradle, JDK 21, and `timeout`.
They compile and test all ten templates with both build tools.
Spring coverage includes web, security, data, and batch projects.
Data tests start the application context with a dedicated H2 database.
Batch tests also start the packaged application and require a completed job.
Separate argument tests pass spaces, apostrophes, quotes, commas, and backslashes through `jav run`.
Gradle also preserves empty arguments. Maven `exec:java` rejects them because its native parser discards them.

## Generated Nix checks

Generate a project and enter its directory.
Follow its README to initialize the dependency lock with `nix run .#lock-deps`.
Run `nix fmt .` before running the generated checks.
Run `nix flake check --no-write-lock-file` after committing the lock files to Git.

The initial Maven project has no `maven-deps.hash`.
The initial Gradle project has an empty `deps.json` for the Nixpkgs update script.
Dependency locking requires network access.
The Java build and test derivations then use the locked dependencies in the Nix sandbox.
The root network integration tests do not initialize generated Nix dependency locks.

Templates accept Java 17 or 21 and numeric Spring Boot 3.x releases.
This version range matches the generated Gradle and Spring configuration.
