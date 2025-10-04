use std::{
    ops::DerefMut,
    sync::{Arc, Mutex},
};

use guppy::graph::PackageGraph;
use pavex_bp_schema::Location;
use pavex_cli_diagnostic::AnnotatedSource;

use super::{
    ComponentKind, OptionalLabeledSpanExt, OptionalSourceSpanExt, ParsedSourceFile, Registration,
    RegistrationKind, config_key_span, f_macro_span, imported_sources_span,
    registration_locations::{attribute_span, impl_header_span, route_path_attr_span},
    registration_span, route_path_span,
};

/// An accumulator for diagnostics.
///
/// The sink can be cheaply cloned, since it's a wrapper around a reference-counted
/// vector.
#[derive(Clone)]
pub struct DiagnosticSink {
    package_graph: PackageGraph,
    diagnostics: Arc<Mutex<Vec<miette::Error>>>,
}

impl DiagnosticSink {
    /// Create a new [`DiagnosticSink`].
    pub fn new(package_graph: PackageGraph) -> Self {
        Self {
            package_graph,
            diagnostics: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Push a new diagnostic into the sink.
    pub fn push<D: miette::Diagnostic + Into<miette::Error>>(&self, diagnostic: D) {
        self.diagnostics
            .lock()
            .expect("The lock around the diagnostic sink was poisoned")
            .push(diagnostic.into());
    }

    /// Drain the sink, retrieving all the diagnostics accumulated so far.
    pub fn drain(&self) -> Vec<miette::Error> {
        std::mem::take(
            self.diagnostics
                .lock()
                .expect("The lock around the diagnostic sink was poisoned")
                .deref_mut(),
        )
    }

    /// Returns `true` if at least one diagnostic with severity "ERROR" has been
    /// pushed into the sink.
    pub fn has_errored(&self) -> bool {
        self.diagnostics
            .lock()
            .expect("The lock around the diagnostic sink was poisoned")
            .iter()
            .any(|e| {
                let severity = e.severity();
                severity == Some(miette::Severity::Error) || severity.is_none()
            })
    }

    /// Check if the sink is empty.
    pub fn is_empty(&self) -> bool {
        self.diagnostics
            .lock()
            .expect("The lock around the diagnostic sink was poisoned")
            .is_empty()
    }

    /// Get the number of diagnostics accumulated so far.
    pub fn len(&self) -> usize {
        self.diagnostics
            .lock()
            .expect("The lock around the diagnostic sink was poisoned")
            .len()
    }
}

/// Source-related methods.
impl DiagnosticSink {
    /// Read and parse the source file that contains the given location.
    pub fn source(&self, location: &Location) -> Option<AnnotatedSource<ParsedSourceFile>> {
        use super::LocationExt as _;

        match location.source_file(&self.package_graph) {
            Ok(s) => Some(AnnotatedSource::new(s)),
            Err(e) => {
                self.push(e);
                None
            }
        }
    }

    /// Return a source file with a label around the given target span.
    pub fn annotated(
        &self,
        target: TargetSpan,
        label_msg: impl Into<String>,
    ) -> Option<AnnotatedSource<ParsedSourceFile>> {
        let s = target.parse_source(self)?;
        let label = match target {
            TargetSpan::Registration(registration, kind) => {
                registration_span(s.source(), registration, kind)
            }
            TargetSpan::Impl(registration) => impl_header_span(s.source(), &registration.location),
            TargetSpan::RoutePath(registration) => match registration.kind {
                RegistrationKind::Blueprint => route_path_span(s.source(), &registration.location),
                RegistrationKind::Attribute => {
                    route_path_attr_span(s.source(), &registration.location)
                }
            },
            TargetSpan::ConfigKeySpan(registration) => config_key_span(s.source(), registration),
            TargetSpan::RawIdentifiers(registration, kind) => match registration.kind {
                RegistrationKind::Attribute => {
                    // TODO: Refine the span to point at the specific attribute/property we care about.
                    attribute_span(s.source(), &registration.location, kind)
                }
                RegistrationKind::Blueprint => f_macro_span(s.source(), &registration.location),
            },
            TargetSpan::ImportedSources(registration) => {
                imported_sources_span(s.source(), &registration.location)
            }
        }
        .labeled(label_msg.into());
        Some(label.attach(s))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TargetSpan<'a> {
    /// A span covering the entire registration expression.
    Registration(&'a Registration, ComponentKind),
    /// A span covering the beginning of an impl block.
    Impl(&'a Registration),
    /// A span covering the route path for a route registration.
    RoutePath(&'a Registration),
    /// A span covering the imported sources for an import.
    ImportedSources(&'a Registration),
    /// A span covering the config key argument specified when registering
    /// a configuration type.
    ConfigKeySpan(&'a Registration),
    /// A span covering the raw identifiers for a component registration.
    ///
    /// This works for both blueprint invocations and macro attributes that
    /// may include raw identifiers as arguments (e.g. an error handler specified
    /// inside a `#[pavex::constructor]` attribute).
    RawIdentifiers(&'a Registration, ComponentKind),
}

impl TargetSpan<'_> {
    /// The absolute path to the source file that should contain the target span.
    pub fn path(&self) -> &str {
        use TargetSpan::*;

        match self {
            Registration(registration, ..)
            | RoutePath(registration)
            | Impl(registration)
            | ImportedSources(registration)
            | ConfigKeySpan(registration)
            | RawIdentifiers(registration, ..) => &registration.location.file,
        }
    }

    /// Try to read and parse the file returned by [`Self::file`].
    pub fn parse_source(
        &self,
        diagnostic_sink: &DiagnosticSink,
    ) -> Option<AnnotatedSource<ParsedSourceFile>> {
        match ParsedSourceFile::new(
            self.path().into(),
            &diagnostic_sink.package_graph.workspace(),
        )
        .map_err(miette::MietteError::IoError)
        {
            Ok(s) => Some(AnnotatedSource::new(s)),
            Err(e) => {
                diagnostic_sink.push(e);
                None
            }
        }
    }
}
