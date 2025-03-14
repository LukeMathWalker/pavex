use ahash::HashMap;
use guppy::graph::PackageGraph;

use crate::compiler::analyses::user_components::UserComponentId;
use crate::diagnostic::CompilerDiagnostic;
use crate::diagnostic::{ComponentKind, TargetSpan};
use crate::language::{ParseError, PathKind, ResolvedPath};

use super::auxiliary::AuxiliaryData;

/// Match the id of a component with its resolved path, if there is one.
///
/// # Implementation notes
///
/// We could do this work while we resolve the identifiers directly to
/// types/callables.
/// We isolate it as its own step to be able to pre-determine which crates
/// we need to compute/fetch JSON docs for since we get higher throughput
/// via a batch computation than by computing them one by one as the need
/// arises.
#[derive(Default)]
pub struct ResolvedPaths(pub HashMap<UserComponentId, ResolvedPath>);

impl ResolvedPaths {
    /// Create a new instance of [`ResolvedPaths`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve all identifiers that have been registered with [`AuxiliaryData`].
    pub fn resolve_all(
        &mut self,
        db: &AuxiliaryData,
        package_graph: &PackageGraph,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
        for (id, _) in db.iter() {
            self.resolve(id, db, package_graph, diagnostics);
        }
    }

    /// Resolve a single raw identifier.
    pub fn resolve(
        &mut self,
        id: UserComponentId,
        db: &AuxiliaryData,
        package_graph: &PackageGraph,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
        let component = &db[id];
        let Some(identifiers_id) = component.raw_identifiers_id() else {
            return;
        };
        let identifiers = &db.identifiers_interner[identifiers_id];
        let kind = match component.kind() {
            ComponentKind::PrebuiltType | ComponentKind::ConfigType => PathKind::Type,
            ComponentKind::RequestHandler
            | ComponentKind::Fallback
            | ComponentKind::Constructor
            | ComponentKind::ErrorHandler
            | ComponentKind::WrappingMiddleware
            | ComponentKind::PostProcessingMiddleware
            | ComponentKind::PreProcessingMiddleware
            | ComponentKind::ErrorObserver => PathKind::Callable,
        };
        match ResolvedPath::parse(identifiers, package_graph, kind) {
            Ok(path) => {
                self.0.insert(id, path);
            }
            Err(e) => invalid_identifiers(e, id, db, diagnostics),
        }
    }
}

fn invalid_identifiers(
    e: ParseError,
    id: UserComponentId,
    db: &AuxiliaryData,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    let label_msg = match &e {
        ParseError::InvalidPath(_) => "The invalid path",
        ParseError::PathMustBeAbsolute(_) => "The relative path",
    };
    let source = diagnostics.annotated(
        TargetSpan::Registration(&db.id2registration[&id]),
        label_msg,
    );
    let help = match &e {
        ParseError::InvalidPath(inner) => {
            inner.raw_identifiers.import_path.strip_suffix("()").map(|stripped| format!("The `f!` macro expects an unambiguous path as input, not a function call. Remove the `()` at the end: `f!({stripped})`"))
        }
        ParseError::PathMustBeAbsolute(_) =>
            Some(
                "If it is a local import, the path must start with `crate::`, `self::` or `super::`.\n\
                If it is an import from a dependency, the path must start with \
                the dependency name (e.g. `dependency::`)."
                .to_string(),
            )
    };
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .optional_help(help)
        .build();
    diagnostics.push(diagnostic);
}
