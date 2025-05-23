use std::collections::BTreeSet;

use guppy::PackageId;
use guppy::graph::PackageGraph;
use pavex_bp_schema::{CreatedAt, Location, Sources};
use syn::Token;
use syn::ext::IdentExt;
use syn::punctuated::Punctuated;

use crate::compiler::analyses::domain::DomainGuard;
use crate::diagnostic::{self, DiagnosticSink, OptionalLabeledSpanExt};
use crate::diagnostic::{CompilerDiagnostic, OptionalSourceSpanExt};
use crate::language::{
    CrateNameResolutionError, UnknownCrate, UnknownDependency, dependency_name2package_id,
    krate2package_id,
};

use super::auxiliary::AuxiliaryData;
use super::{ScopeId, UserComponentId};

/// A user-registered import that's yet to be processed.
pub struct UnresolvedImport {
    /// The scope to which the imported components will belong.
    pub scope_id: ScopeId,
    /// The sources being imported.
    pub sources: Sources,
    /// The location where the import was created.
    pub created_at: CreatedAt,
    /// The location at which the import was registered against the blueprint.
    pub registered_at: Location,
    /// Which component kinds are being imported.
    pub kind: ImportKind,
}

/// The kind of imports exposed by Pavex to the user.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImportKind {
    /// Constructors, configuration types, prebuilt types, error handlers.
    OrderIndependentComponents,
    /// Request handlers.
    Routes {
        path_prefix: Option<String>,
        domain_guard: Option<DomainGuard>,
        observer_chain: Vec<UserComponentId>,
        middleware_chain: Vec<UserComponentId>,
    },
}

/// A normalized import path.
#[derive(Debug, Clone)]
pub struct ResolvedImport {
    /// The path of the imported module.
    ///
    /// # Invariants
    ///
    /// The first segment must match the name of the package in [`ResolvedImport::package_id`].
    pub path: Vec<String>,
    /// The ID of the package that defines the imported module.
    pub package_id: PackageId,
    /// The scope to which the imported components will belong.
    pub scope_id: ScopeId,
    /// Which component kinds are being imported.
    pub kind: ImportKind,
}

/// For each import:
///
/// - Convert relative imported module paths into absolute paths.
/// - Match the path root to a package ID in the package graph.
///
/// We also resolve `*` imports to the actual set of packages they are supposed to match.
pub(super) fn resolve_imports(
    db: &AuxiliaryData,
    package_graph: &PackageGraph,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) -> Vec<(ResolvedImport, usize)> {
    let mut resolved_imports = Vec::new();
    for (import_id, raw_import) in db.imports.iter().enumerate() {
        let imported_in = match krate2package_id(
            &raw_import.created_at.package_name,
            &raw_import.created_at.package_version,
            package_graph,
        ) {
            Ok(package_id) => package_id,
            Err(e) => {
                unknown_registration_crate(e, raw_import, diagnostics);
                continue;
            }
        };
        match &raw_import.sources {
            Sources::All => {
                let sources = sources_for_all(&imported_in, package_graph);
                for source in sources {
                    let name = package_graph
                        .metadata(&source)
                        .unwrap()
                        .name()
                        .replace('-', "_");
                    resolved_imports.push((
                        ResolvedImport {
                            path: vec![name],
                            package_id: source,
                            scope_id: raw_import.scope_id,
                            kind: raw_import.kind.clone(),
                        },
                        import_id,
                    ));
                }
            }
            Sources::Some(sources) => {
                for source in sources {
                    let mut path = match syn::parse_str::<RawModulePath>(source) {
                        Ok(p) => p,
                        Err(e) => {
                            invalid_module_path(e, source.into(), raw_import, diagnostics);
                            continue;
                        }
                    };
                    path.make_absolute(&raw_import.created_at);
                    let package_name = path.0.first().expect("Module path can't be empty");
                    let package_id =
                        match dependency_name2package_id(package_name, &imported_in, package_graph)
                        {
                            Ok(package_id) => package_id,
                            Err(e) => {
                                crate_resolution_error(e, raw_import, diagnostics);
                                continue;
                            }
                        };
                    resolved_imports.push((
                        ResolvedImport {
                            path: path.0,
                            package_id,
                            scope_id: raw_import.scope_id,
                            kind: raw_import.kind.clone(),
                        },
                        import_id,
                    ));
                }
            }
        }
    }
    resolved_imports
}

/// The set of package IDs that are in scope if the user imported `*` from `current_package_id`.
fn sources_for_all(current_package_id: &PackageId, graph: &PackageGraph) -> BTreeSet<PackageId> {
    graph
        .metadata(current_package_id)
        .unwrap()
        .direct_links()
        .filter(|link| {
            // It shouldn't be a dev or build dependency.
            link.normal().is_present()
            &&
            // And it must depend on `pavex`, otherwise it can't contain annotated components.
            // This is more an optimisation than a requirement, since we can always fall back to
            // scanning the docs for _all_ direct dependencies, but it'd be wasteful.
            link
                .to()
                .direct_links()
                .any(|l| l.to().name() == "pavex" && l.normal().is_present())
        })
        .map(|l| l.to().id().to_owned())
        .collect()
}

/// A raw module path, parsed from the serialized source.
struct RawModulePath(Vec<String>);

impl RawModulePath {
    /// Replace any relative path components (e.g. `super`, `self`, `crate`) with
    /// their absolute counterparts.
    fn make_absolute(&mut self, created_at: &CreatedAt) {
        let first = self
            .0
            .first()
            .expect("An empty module path in an import")
            .to_owned();
        match first.as_str() {
            "crate" => {
                self.0[0] = created_at.package_name.clone();
            }
            "self" => {
                let old_segments = std::mem::take(&mut self.0);
                let new_segments = created_at
                    .module_path
                    .split("::")
                    .map(|s| s.trim().to_owned())
                    .chain(old_segments.into_iter().skip(1))
                    .collect();
                self.0 = new_segments;
            }
            "super" => {
                // We make the path absolute by adding replacing `super` with the relevant
                // parts of the module path.
                let n_super = self.0.iter().filter(|s| s.as_str() == "super").count();
                let old_segments = std::mem::take(&mut self.0);
                // The path is relative to the current module.
                // We "rebase" it to get an absolute path.
                let module_segments: Vec<_> = created_at
                    .module_path
                    .split("::")
                    .map(|s| s.trim().to_owned())
                    .collect();
                let n_module_segments = module_segments.len();
                let new_segments: Vec<_> = module_segments
                    .into_iter()
                    .take(n_module_segments - n_super)
                    .chain(old_segments.into_iter().skip(n_super))
                    .collect();
                self.0 = new_segments;
            }
            _ => {}
        }
    }
}

impl syn::parse::Parse for RawModulePath {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let p = Punctuated::<syn::Ident, Token![::]>::parse_separated_nonempty_with(
            input,
            // We use `parse_any` to allow keywords (e.g. `crate` or `super` or `self`)
            // which would otherwise be rejected.
            syn::Ident::parse_any,
        )?;
        Ok(Self(p.into_iter().map(|ident| ident.to_string()).collect()))
    }
}

fn crate_resolution_error(
    e: CrateNameResolutionError,
    import: &UnresolvedImport,
    diagnostics: &mut DiagnosticSink,
) {
    match e {
        CrateNameResolutionError::UnknownDependency(e) => {
            unknown_dependency_crate(e, import, diagnostics);
        }
        CrateNameResolutionError::UnknownCrateName(e) => {
            unknown_registration_crate(e, import, diagnostics);
        }
    }
}

fn unknown_registration_crate(
    e: UnknownCrate,
    import: &UnresolvedImport,
    diagnostics: &mut DiagnosticSink,
) {
    #[derive(Debug, thiserror::Error)]
    #[error(
        "{source}.\n\
        You tried to import from `{name}`, but I can't resolve that import if I can't match `{name}` to a package in your dependency tree."
    )]
    struct CannotMatchPackageIdToCrateName {
        name: String,
        source: UnknownCrate,
    }

    let source = diagnostics.source(&import.registered_at).map(|s| {
        let msg = match import.kind {
            ImportKind::OrderIndependentComponents => "The import was registered here",
            ImportKind::Routes { .. } => "The routes were imported here",
        };
        diagnostic::imported_sources_span(s.source(), &import.registered_at)
            .labeled(msg.into())
            .attach(s)
    });
    let diagnostic = CompilerDiagnostic::builder(CannotMatchPackageIdToCrateName {
        name: e.name.clone(),
        source: e,
    })
    .optional_source(source)
    .build();
    diagnostics.push(diagnostic);
}

fn unknown_dependency_crate(
    e: UnknownDependency,
    import: &UnresolvedImport,
    diagnostics: &mut DiagnosticSink,
) {
    #[derive(Debug, thiserror::Error)]
    #[error(
        "You tried to import from `{dependency}`, but `{dependent}` has no direct dependency named `{dependency}`."
    )]
    struct CannotFindDependency {
        dependent: String,
        dependency: String,
        source: UnknownDependency,
    }

    let source = diagnostics.source(&import.registered_at).map(|s| {
        let msg = match import.kind {
            ImportKind::OrderIndependentComponents => "The import was registered here",
            ImportKind::Routes { .. } => "The routes were imported here",
        };
        diagnostic::imported_sources_span(s.source(), &import.registered_at)
            .labeled(msg.into())
            .attach(s)
    });
    let diagnostic = CompilerDiagnostic::builder(CannotFindDependency {
        dependent: e.dependent_name.clone(),
        dependency: e.dependency_name.clone(),
        source: e,
    })
    .optional_source(source)
    .help("Check your `Cargo.toml` file for typos or missing dependencies.".into())
    .help(
        "The path must start with either `crate` or `super` if you want to import from a local module."
            .into(),
    )
    .build();
    diagnostics.push(diagnostic);
}

fn invalid_module_path(
    e: syn::Error,
    raw_path: String,
    import: &UnresolvedImport,
    diagnostics: &mut DiagnosticSink,
) {
    #[derive(Debug, thiserror::Error)]
    #[error("`{path}` is not a valid import path.")]
    struct InvalidModulePath {
        path: String,
        source: syn::Error,
    }

    let source = diagnostics.source(&import.registered_at).map(|s| {
        let msg = match import.kind {
            ImportKind::OrderIndependentComponents => "The import was registered here",
            ImportKind::Routes { .. } => "The routes were imported here",
        };
        diagnostic::imported_sources_span(s.source(), &import.registered_at)
            .labeled(msg.into())
            .attach(s)
    });
    let diagnostic = CompilerDiagnostic::builder(InvalidModulePath { path: raw_path, source: e })
        .optional_source(source)
        .help("Did you use the `from!` macro to register your sources? An invalid module path should have been caught earlier in the compilation process.".into())
        .build();
    diagnostics.push(diagnostic);
}
