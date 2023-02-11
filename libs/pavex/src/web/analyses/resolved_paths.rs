use ahash::{HashMap, HashMapExt};
use guppy::graph::PackageGraph;

use crate::diagnostic;
use crate::diagnostic::{CompilerDiagnostic, LocationExt, OptionalSourceSpanExt};
use crate::language::{ParseError, ResolvedPath};
use crate::web::analyses::raw_identifiers::RawCallableIdentifiersDb;
use crate::web::analyses::user_components::{UserComponentDb, UserComponentId};
use crate::web::interner::Interner;

pub(crate) type ResolvedPathId = la_arena::Idx<ResolvedPath>;

pub(crate) struct ResolvedPathDb {
    interner: Interner<ResolvedPath>,
    component_id2path_id: HashMap<UserComponentId, ResolvedPathId>,
}

impl ResolvedPathDb {
    pub fn build(
        component_db: &UserComponentDb,
        raw_callable_identifiers_db: &RawCallableIdentifiersDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Self {
        let mut interner = Interner::new();
        let mut component_id2path_id = HashMap::new();
        for (component_id, component) in component_db.iter() {
            let raw_callable_identifiers_id = component.raw_callable_identifiers_id();
            let raw_callable_identifiers =
                &raw_callable_identifiers_db[raw_callable_identifiers_id];
            match ResolvedPath::parse(raw_callable_identifiers, package_graph) {
                Ok(path) => {
                    let path_id = interner.get_or_intern(path);
                    component_id2path_id.insert(component_id, path_id);
                }
                Err(e) => Self::capture_diagnostics(
                    e,
                    component_id,
                    component_db,
                    raw_callable_identifiers_db,
                    package_graph,
                    diagnostics,
                ),
            }
        }
        Self {
            interner,
            component_id2path_id,
        }
    }

    fn capture_diagnostics(
        e: ParseError,
        component_id: UserComponentId,
        component_db: &UserComponentDb,
        raw_identifiers_db: &RawCallableIdentifiersDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let raw_identifier_id = component_db[component_id].raw_callable_identifiers_id();
        let location = raw_identifiers_db.get_location(raw_identifier_id);
        let source = match location.source_file(package_graph) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(e.into());
                return;
            }
        };
        let source_span = diagnostic::get_f_macro_invocation_span(&source, location);
        let (label, help) = match &e {
            ParseError::InvalidPath(_) => ("The invalid import path was registered here", None),
            ParseError::PathMustBeAbsolute(_) => (
                "The relative import path was registered here",
                Some(
                    "If it is a local import, the path must start with `crate::`.\n\
                    If it is an import from a dependency, the path must start with \
                    the dependency name (e.g. `dependency::`).",
                ),
            ),
        };
        let diagnostic = CompilerDiagnostic::builder(source, e)
            .optional_label(source_span.labeled(label.into()))
            .optional_help(help.map(ToOwned::to_owned))
            .build();
        diagnostics.push(diagnostic.into());
    }
}

impl std::ops::Index<ResolvedPathId> for ResolvedPathDb {
    type Output = ResolvedPath;

    fn index(&self, index: ResolvedPathId) -> &Self::Output {
        &self.interner[index]
    }
}

impl std::ops::Index<UserComponentId> for ResolvedPathDb {
    type Output = ResolvedPath;

    fn index(&self, index: UserComponentId) -> &Self::Output {
        &self[self.component_id2path_id[&index]]
    }
}
