use std::fmt::{Display, Formatter};

use guppy::graph::{PackageMetadata, PackageSource};
use guppy::Version;

/// A selector that follows the [package ID specification](https://doc.rust-lang.org/cargo/reference/pkgid-spec.html).
/// It is used as argument to the `-p`/`--package` flag in `cargo`'s commands.
#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub struct PackageIdSpecification {
    pub(super) source: Option<String>,
    pub(super) name: String,
    pub(super) version: Option<Version>,
}

impl PackageIdSpecification {
    pub fn new(metadata: &PackageMetadata) -> Self {
        let source = match metadata.source() {
            PackageSource::Workspace(source) | PackageSource::Path(source) => {
                let source = source.strip_prefix("path+").unwrap_or(source);
                if source.as_str().is_empty() {
                    source.to_string()
                } else {
                    let source = if source.is_relative() {
                        metadata.graph().workspace().root().join(source).to_string()
                    } else {
                        source.to_string()
                    };
                    format!("file:///{}", source)
                }
            }
            PackageSource::External(source) => {
                let s = if let Some(source) = source.strip_prefix("git+") {
                    source
                } else if let Some(source) = source.strip_prefix("registry+") {
                    source
                } else {
                    source
                };
                s.to_owned()
            }
        };
        let source = if source.is_empty() {
            None
        } else {
            Some(source)
        };
        let name = metadata.name().to_owned();
        let version = Some(metadata.version().to_owned());
        Self {
            source,
            name,
            version,
        }
    }
}

impl Display for PackageIdSpecification {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(source) = &self.source {
            write!(f, "{}#", source)?;
        }
        write!(f, "{}", &self.name)?;
        if let Some(version) = &self.version {
            write!(f, "@{}", version)?;
        }
        Ok(())
    }
}
