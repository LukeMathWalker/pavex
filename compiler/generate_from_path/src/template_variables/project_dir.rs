use crate::GenerateArgs;
use anyhow::bail;
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

/// Stores user inputted name and provides convenience methods
/// for handling casing.
pub struct ProjectDir(PathBuf);

impl AsRef<Path> for ProjectDir {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

impl Display for ProjectDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.display().fmt(f)
    }
}

impl TryFrom<&GenerateArgs> for ProjectDir {
    type Error = anyhow::Error;

    fn try_from(args: &GenerateArgs) -> Result<Self, Self::Error> {
        let project_dir = args.destination.join(&args.name);

        if project_dir.exists() {
            bail!("Target directory already exists, aborting!");
        }

        Ok(Self(project_dir))
    }
}
