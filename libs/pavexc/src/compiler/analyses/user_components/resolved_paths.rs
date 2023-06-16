use ahash::{HashMap, HashMapExt};
use guppy::graph::PackageGraph;

use crate::compiler::analyses::user_components::raw_db::RawUserComponentDb;
use crate::compiler::analyses::user_components::UserComponentId;
use crate::compiler::interner::Interner;
use crate::diagnostic;
use crate::diagnostic::{CompilerDiagnostic, LocationExt, OptionalSourceSpanExt};
use crate::language::{ParseError, ResolvedPath};

pub(super) type ResolvedPathId = la_arena::Idx<ResolvedPath>;

pub(super) struct ResolvedPathDb {
    interner: Interner<ResolvedPath>,
    component_id2path_id: HashMap<UserComponentId, ResolvedPathId>,
}

impl ResolvedPathDb {
    #[tracing::instrument("Build resolved path database", skip_all, level = "trace")]
    pub fn build(
        component_db: &RawUserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Self {
        let mut interner = Interner::new();
        let mut component_id2path_id = HashMap::new();
        for (component_id, component) in component_db.iter() {
            let raw_callable_identifiers = component.raw_callable_identifiers(component_db);
            match ResolvedPath::parse(raw_callable_identifiers, package_graph) {
                Ok(path) => {
                    let path_id = interner.get_or_intern(path);
                    component_id2path_id.insert(component_id, path_id);
                }
                Err(e) => Self::capture_diagnostics(
                    e,
                    component_id,
                    component_db,
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

    /// Iterate over all the resolved paths in the database, returning their id and the associated
    /// `ResolvedPath`.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (ResolvedPathId, &ResolvedPath)> + ExactSizeIterator + DoubleEndedIterator
    {
        self.interner.iter()
    }

    fn capture_diagnostics(
        e: ParseError,
        component_id: UserComponentId,
        component_db: &RawUserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = component_db.get_location(component_id);
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
