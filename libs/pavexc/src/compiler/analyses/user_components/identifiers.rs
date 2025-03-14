use ahash::{HashMap, HashMapExt};
use guppy::graph::PackageGraph;

use crate::compiler::analyses::user_components::UserComponentId;
use crate::diagnostic::{self, ComponentKind, OptionalLabeledSpanExt};
use crate::diagnostic::{CompilerDiagnostic, OptionalSourceSpanExt};
use crate::language::{ParseError, PathKind, ResolvedPath};

use super::auxiliary::AuxiliaryData;

/// Return a mapping from identifiers to their resolved counterpart.
///
/// We could do this work while we resolve the identifiers directly to
/// types/callables.
/// We isolate it as its own step to be able to pre-determine which crates
/// we need to compute/fetch JSON docs for since we get higher throughput
/// via a batch computation than by computing them one by one as the need
/// arises.
pub(super) fn resolve_raw_identifiers(
    db: &AuxiliaryData,
    package_graph: &PackageGraph,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) -> HashMap<UserComponentId, ResolvedPath> {
    let mut component_id2path = HashMap::new();
    for (component_id, component) in db.iter() {
        let Some(bp_source) = component.bp_source() else {
            continue;
        };
        let identifiers = &db.identifiers_interner[bp_source.identifiers_id];
        let kind = match component.kind() {
            ComponentKind::PrebuiltType | ComponentKind::ConfigType => PathKind::Type,
            ComponentKind::RequestHandler
            | ComponentKind::Constructor
            | ComponentKind::ErrorHandler
            | ComponentKind::WrappingMiddleware
            | ComponentKind::PostProcessingMiddleware
            | ComponentKind::PreProcessingMiddleware
            | ComponentKind::ErrorObserver => PathKind::Callable,
        };
        match ResolvedPath::parse(identifiers, package_graph, kind) {
            Ok(path) => {
                component_id2path.insert(component_id, path);
            }
            Err(e) => capture_diagnostics(e, component_id, db, diagnostics),
        }
    }
    component_id2path
}

fn capture_diagnostics(
    e: ParseError,
    id: UserComponentId,
    db: &AuxiliaryData,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    let location = &db.id2locations[&id];
    let source = diagnostics.source(location).map(|s| {
        let span = diagnostic::f_macro_span(s.source(), location);
        let label_msg = match &e {
            ParseError::InvalidPath(_) => "The invalid import path was registered here",
            ParseError::PathMustBeAbsolute(_) => "The relative import path was registered here",
        };
        span.labeled(label_msg.into()).attach(s)
    });
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
