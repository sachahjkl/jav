pub mod detect;
pub mod sources;

pub fn project_command(kind: ProjectKind) -> anyhow::Result<String> {
    project_command_in(kind, &std::env::current_dir()?)
}

fn project_command_in(kind: ProjectKind, root: &std::path::Path) -> anyhow::Result<String> {
    let (wrapper, tool) = match kind {
        ProjectKind::Maven => (if cfg!(windows) { "mvnw.cmd" } else { "mvnw" }, "mvn"),
        ProjectKind::Gradle => (
            if cfg!(windows) {
                "gradlew.bat"
            } else {
                "gradlew"
            },
            "gradle",
        ),
        ProjectKind::Simple => return Ok("javac".into()),
    };
    for directory in root.ancestors() {
        if directory.join(wrapper).is_file() {
            if directory == root {
                return Ok(format!(".{}{wrapper}", std::path::MAIN_SEPARATOR));
            }
            let path = dunce::canonicalize(directory)?.join(wrapper);
            return path
                .into_os_string()
                .into_string()
                .map_err(|_| anyhow::anyhow!("wrapper path is not valid UTF-8"));
        }
        // A .git file also marks a worktree or submodule boundary.
        if directory.join(".git").exists() {
            break;
        }
    }
    Ok(tool.into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectKind {
    Maven,
    Gradle,
    Simple,
}

impl ProjectKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Maven => "Maven",
            Self::Gradle => "Gradle",
            Self::Simple => "simple Java",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;

    #[test]
    fn selects_parent_wrapper_but_prefers_local_wrapper() {
        for (kind, name) in [
            (
                ProjectKind::Maven,
                if cfg!(windows) { "mvnw.cmd" } else { "mvnw" },
            ),
            (
                ProjectKind::Gradle,
                if cfg!(windows) {
                    "gradlew.bat"
                } else {
                    "gradlew"
                },
            ),
        ] {
            let root = assert_fs::TempDir::new().unwrap();
            root.child(".git").create_dir_all().unwrap();
            root.child(name).touch().unwrap();
            let module = root.child("module");
            module.create_dir_all().unwrap();
            assert_eq!(
                std::path::Path::new(&project_command_in(kind, module.path()).unwrap())
                    .canonicalize()
                    .unwrap(),
                root.path().canonicalize().unwrap().join(name)
            );
            module.child(name).touch().unwrap();
            assert_eq!(
                project_command_in(kind, module.path()).unwrap(),
                format!(".{}{name}", std::path::MAIN_SEPARATOR)
            );
        }
    }

    #[test]
    fn parent_search_stops_at_git_directory_or_file() {
        for git_file in [false, true] {
            let root = assert_fs::TempDir::new().unwrap();
            let name = if cfg!(windows) { "mvnw.cmd" } else { "mvnw" };
            root.child(name).touch().unwrap();
            let repository = root.child("repository");
            repository.child("module").create_dir_all().unwrap();
            if git_file {
                repository.child(".git").touch().unwrap();
            } else {
                repository.child(".git").create_dir_all().unwrap();
            }
            assert_eq!(
                project_command_in(ProjectKind::Maven, repository.child("module").path()).unwrap(),
                "mvn"
            );
        }
    }

    #[test]
    fn prefers_platform_wrapper_and_otherwise_uses_path_tool() {
        let root = assert_fs::TempDir::new().unwrap();
        assert_eq!(
            project_command_in(ProjectKind::Maven, root.path()).unwrap(),
            "mvn"
        );
        let wrapper = if cfg!(windows) { "mvnw.cmd" } else { "mvnw" };
        root.child(wrapper).touch().unwrap();
        assert_eq!(
            project_command_in(ProjectKind::Maven, root.path()).unwrap(),
            format!(".{}{wrapper}", std::path::MAIN_SEPARATOR)
        );
        let wrapper = if cfg!(windows) {
            "gradlew.bat"
        } else {
            "gradlew"
        };
        root.child(wrapper).touch().unwrap();
        assert_eq!(
            project_command_in(ProjectKind::Gradle, root.path()).unwrap(),
            format!(".{}{wrapper}", std::path::MAIN_SEPARATOR)
        );
    }
}
