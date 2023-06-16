use std::path::Path;

use guppy::graph::PackageGraph;
use miette::{MietteError, NamedSource};

use pavex::blueprint::reflection::Location;

#[derive(Debug, Clone)]
pub struct ParsedSourceFile {
    pub(crate) path: std::path::PathBuf,
    pub(crate) contents: String,
    pub(crate) parsed: syn::File,
}

impl ParsedSourceFile {
    pub fn new(
        path: std::path::PathBuf,
        workspace: &guppy::graph::Workspace,
    ) -> Result<Self, std::io::Error> {
        let source = read_source_file(&path, workspace)?;
        let parsed = syn::parse_str(&source).unwrap();
        Ok(Self {
            path,
            contents: source,
            parsed,
        })
    }
}

impl From<ParsedSourceFile> for NamedSource {
    fn from(f: ParsedSourceFile) -> Self {
        let file_name = f.path.to_string_lossy();
        NamedSource::new(file_name, f.contents)
    }
}

/// Given a file path, return the content of the source file it refers to.
///
/// Relative paths are assumed to be relative to the workspace root manifest.  
/// Absolute paths are used as-is.
pub fn read_source_file(
    path: &Path,
    workspace: &guppy::graph::Workspace,
) -> Result<String, std::io::Error> {
    if path.is_absolute() {
        fs_err::read_to_string(path)
    } else {
        let path = workspace.root().as_std_path().join(path);
        fs_err::read_to_string(path)
    }
}

pub trait LocationExt {
    /// Read the source file that contains this location.
    fn source_file(&self, package_graph: &PackageGraph) -> Result<ParsedSourceFile, MietteError>;
}

impl LocationExt for Location {
    fn source_file(&self, package_graph: &PackageGraph) -> Result<ParsedSourceFile, MietteError> {
        ParsedSourceFile::new(self.file.as_str().into(), &package_graph.workspace())
            .map_err(MietteError::IoError)
    }
}
