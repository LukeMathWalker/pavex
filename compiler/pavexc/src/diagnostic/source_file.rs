use std::path::Path;

use guppy::graph::PackageGraph;
use miette::{MietteError, NamedSource};

use pavex_bp_schema::Location;
use relative_path::PathExt;

#[derive(Debug, Clone)]
pub struct ParsedSourceFile {
    pub(crate) display_path: String,
    pub(crate) contents: String,
    pub(crate) parsed: syn::File,
}

impl ParsedSourceFile {
    pub fn new(
        path: std::path::PathBuf,
        workspace: &guppy::graph::Workspace,
    ) -> Result<Self, std::io::Error> {
        let display_path = if path.is_absolute() {
            if let Ok(relative) = path.relative_to(workspace.root()) {
                relative.to_string()
            } else {
                path.display().to_string()
            }
        } else {
            path.display().to_string()
        };
        let source = read_source_file(&path, workspace)?;
        let parsed = syn::parse_str(&source).unwrap();
        Ok(Self {
            display_path,
            contents: source,
            parsed,
        })
    }
}

impl From<ParsedSourceFile> for NamedSource<String> {
    fn from(f: ParsedSourceFile) -> Self {
        NamedSource::new(f.display_path, f.contents)
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
