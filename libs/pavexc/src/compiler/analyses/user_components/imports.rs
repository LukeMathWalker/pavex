use std::collections::BTreeSet;

use guppy::PackageId;
use guppy::graph::PackageGraph;
use pavex_bp_schema::{CreatedAt, Import, Sources};
use syn::Token;
use syn::ext::IdentExt;
use syn::punctuated::Punctuated;

use crate::diagnostic::{self, DiagnosticSink, OptionalLabeledSpanExt};
use crate::diagnostic::{CompilerDiagnostic, OptionalSourceSpanExt};
use crate::language::{
    CrateNameResolutionError, UnknownCrate, UnknownDependency, dependency_name2package_id,
    krate2package_id,
};

use super::auxiliary::AuxiliaryData;

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
    for (import_id, (import, _)) in db.imports.iter().enumerate() {
        let imported_in = match krate2package_id(
            &import.created_at.package_name,
            &import.created_at.package_version,
            package_graph,
        ) {
            Ok(package_id) => package_id,
            Err(e) => {
                unknown_registration_crate(e, import, diagnostics);
                continue;
            }
        };
        match &import.sources {
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
                            invalid_module_path(e, source.into(), import, diagnostics);
                            continue;
                        }
                    };
                    path.make_absolute(&import.created_at);
                    let package_name = path.0.first().expect("Module path can't be empty");
                    let package_id =
                        match dependency_name2package_id(package_name, &imported_in, package_graph)
                        {
                            Ok(package_id) => package_id,
                            Err(e) => {
                                crate_resolution_error(e, import, diagnostics);
                                continue;
                            }
                        };
                    resolved_imports.push((
                        ResolvedImport {
                            path: path.0,
                            package_id,
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
    import: &Import,
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

fn unknown_registration_crate(e: UnknownCrate, import: &Import, diagnostics: &mut DiagnosticSink) {
    #[derive(Debug, thiserror::Error)]
    #[error(
        "{source}.\n\
        You registered an import against your blueprint in `{name}`. \
        I can't resolve that import until I can match `{name}` to a package in your dependency tree."
    )]
    struct CannotMatchPackageIdToCrateName {
        name: String,
        source: UnknownCrate,
    }

    let source = diagnostics.source(&import.registered_at).map(|s| {
        diagnostic::imported_sources_span(s.source(), &import.registered_at)
            .labeled("The import was registered here".into())
            .attach(s)
    });
    let diagnostic = CompilerDiagnostic::builder(CannotMatchPackageIdToCrateName { name: e.name.clone(), source: e })
        .optional_source(source)
        .help("Did you use the `from!` macro to register your sources? Setting `WithLocation`'s fields manually is bound to cause problems for Pavex.".into())
        .build();
    diagnostics.push(diagnostic);
}

fn unknown_dependency_crate(
    e: UnknownDependency,
    import: &Import,
    diagnostics: &mut DiagnosticSink,
) {
    #[derive(Debug, thiserror::Error)]
    #[error(
        "You tried to import items from `{dependency}`, but `{dependent}` has no direct dependency named `{dependency}`."
    )]
    struct CannotFindDependency {
        dependent: String,
        dependency: String,
        source: UnknownDependency,
    }

    let source = diagnostics.source(&import.registered_at).map(|s| {
        diagnostic::imported_sources_span(s.source(), &import.registered_at)
            .labeled("The import was registered here".into())
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
        "The path must start with either `crate` or `super` if you want to import a local module."
            .into(),
    )
    .build();
    diagnostics.push(diagnostic);
}

fn invalid_module_path(
    e: syn::Error,
    raw_path: String,
    import: &Import,
    diagnostics: &mut DiagnosticSink,
) {
    #[derive(Debug, thiserror::Error)]
    #[error("`{path}` is not a valid import path.")]
    struct InvalidModulePath {
        path: String,
        source: syn::Error,
    }

    let source = diagnostics.source(&import.registered_at).map(|s| {
        diagnostic::imported_sources_span(s.source(), &import.registered_at)
            .labeled("The import was registered here".into())
            .attach(s)
    });
    let diagnostic = CompilerDiagnostic::builder(InvalidModulePath { path: raw_path, source: e })
        .optional_source(source)
        .help("Did you use the `from!` macro to register your sources? An invalid module path should have been caught earlier.".into())
        .build();
    diagnostics.push(diagnostic);
}
