use guppy::graph::PackageGraph;
use pavex_bp_schema::Location;
use pavex_cli_diagnostic::AnnotatedSource;

use super::ParsedSourceFile;

/// An accumulator for diagnostics.
pub struct DiagnosticSink {
    package_graph: PackageGraph,
    diagnostics: Vec<miette::Error>,
}

impl DiagnosticSink {
    /// Create a new [`DiagnosticSink`].
    pub fn new(package_graph: PackageGraph) -> Self {
        Self {
            package_graph,
            diagnostics: Vec::new(),
        }
    }

    /// Push a new diagnostic into the sink.
    pub fn push<D: miette::Diagnostic + Into<miette::Error>>(&mut self, diagnostic: D) {
        self.diagnostics.push(diagnostic.into());
    }

    /// Get the diagnostics accumulated so far.
    pub fn diagnostics(&self) -> &[miette::Error] {
        &self.diagnostics
    }

    /// Check if the sink is empty.
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Get the number of diagnostics accumulated so far.
    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }
}

/// Source-related methods.
impl DiagnosticSink {
    /// Read and parse the source file that contains the given location.
    pub fn source(&mut self, location: &Location) -> Option<AnnotatedSource<ParsedSourceFile>> {
        use super::LocationExt as _;

        match location.source_file(&self.package_graph) {
            Ok(s) => Some(AnnotatedSource::new(s)),
            Err(e) => {
                self.push(e);
                None
            }
        }
    }
}
