pub mod detect;
pub mod sources;

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
