use std::fmt::Debug;
use std::io::{BufWriter, Write};
use std::ops::Deref;
use std::path::Path;

use ahash::HashSet;
use bimap::BiHashMap;
use guppy::graph::PackageGraph;
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};
use miette::miette;
use proc_macro2::Ident;
use quote::format_ident;

use pavex_builder::{constructor::Lifecycle, Blueprint};

use crate::compiler::analyses::call_graph::{
    application_state_call_graph, handler_call_graph, ApplicationStateCallGraph, CallGraph,
};
use crate::compiler::analyses::components::{ComponentDb, ComponentId, HydratedComponent};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::compiler::analyses::user_components::{RouterKey, UserComponentDb};
use crate::compiler::computation::Computation;
use crate::compiler::generated_app::GeneratedApp;
use crate::compiler::resolvers::CallableResolutionError;
use crate::compiler::traits::{assert_trait_is_implemented, MissingTraitImplementationError};
use crate::compiler::utils::process_framework_path;
use crate::compiler::{codegen, route_parameter_validation};
use crate::diagnostic;
use crate::diagnostic::{CompilerDiagnostic, LocationExt, SourceSpanExt};
use crate::language::ResolvedType;
use crate::rustdoc::{CrateCollection, TOOLCHAIN_CRATES};

pub(crate) const GENERATED_APP_PACKAGE_ID: &str = "crate";

/// An in-memory representation that can be used to generate application code that matches
/// the constraints and instructions from a [`Blueprint`] instance.
pub struct App {
    package_graph: PackageGraph,
    handler_call_graphs: IndexMap<RouterKey, CallGraph>,
    application_state_call_graph: ApplicationStateCallGraph,
    runtime_singleton_bindings: BiHashMap<Ident, ResolvedType>,
    request_scoped_framework_bindings: BiHashMap<Ident, ResolvedType>,
    codegen_types: HashSet<ResolvedType>,
    component_db: ComponentDb,
    computation_db: ComputationDb,
}

#[tracing::instrument]
fn compute_package_graph() -> Result<PackageGraph, miette::Error> {
    // `cargo metadata` seems to be the only reliable way of retrieving the path to
    // the root manifest of the current workspace for a Rust project.
    guppy::MetadataCommand::new()
        .exec()
        .map_err(|e| miette!(e))?
        .build_graph()
        .map_err(|e| miette!(e))
}

impl App {
    #[tracing::instrument(skip_all)]
    /// Process the [`Blueprint`] created by user into an [`App`] instance—an in-memory
    /// representation that can be used to generate application code that matches the constraints
    /// and instructions in the blueprint.
    ///
    /// Many different things can go wrong during this process: this method tries its best to
    /// report all errors to the user, but it may not be able to do so in all cases.
    pub fn build(bp: Blueprint) -> Result<Self, Vec<miette::Error>> {
        /// Exit early if there is at least one error.
        macro_rules! exit_on_errors {
            ($var:ident) => {
                if !$var.is_empty() {
                    return Err($var);
                }
            };
        }

        let package_graph = compute_package_graph().map_err(|e| vec![e])?;
        let krate_collection = CrateCollection::new(package_graph.clone());
        let mut diagnostics = vec![];
        let mut computation_db = ComputationDb::new();
        let Ok(user_component_db) = UserComponentDb::build(
            &bp,
            &mut computation_db,
            &package_graph,
            &krate_collection,
            &mut diagnostics,
        ) else {
            return Err(diagnostics);
        };
        let mut component_db = ComponentDb::build(
            user_component_db,
            &mut computation_db,
            &package_graph,
            &krate_collection,
            &mut diagnostics,
        );
        exit_on_errors!(diagnostics);
        let request_scoped_framework_bindings =
            framework_bindings(&package_graph, &krate_collection);
        let mut constructible_db = ConstructibleDb::build(
            &mut component_db,
            &mut computation_db,
            &package_graph,
            &krate_collection,
            &request_scoped_framework_bindings.right_values().collect(),
            &mut diagnostics,
        );
        exit_on_errors!(diagnostics);
        let handler_call_graphs = {
            let router = component_db.router();
            let mut handler_call_graphs = IndexMap::with_capacity(router.len());
            for (router_key, handler_id) in router {
                let call_graph = handler_call_graph(
                    *handler_id,
                    &computation_db,
                    &component_db,
                    &constructible_db,
                );
                handler_call_graphs.insert(router_key.to_owned(), call_graph);
            }
            handler_call_graphs
        };
        route_parameter_validation::verify_route_parameters(
            &handler_call_graphs,
            &computation_db,
            &component_db,
            &package_graph,
            &krate_collection,
            &mut diagnostics,
        );

        let runtime_singletons: IndexSet<(ResolvedType, ComponentId)> =
            get_required_singleton_types(
                handler_call_graphs.iter(),
                &request_scoped_framework_bindings,
                &constructible_db,
                &component_db,
            );

        verify_singletons(
            &runtime_singletons,
            &component_db,
            &computation_db,
            &package_graph,
            &krate_collection,
            &mut diagnostics,
        );
        let runtime_singleton_bindings = runtime_singletons
            .iter()
            .enumerate()
            // Assign a unique name to each singleton
            .map(|(i, (type_, _))| (format_ident!("s{}", i), type_.to_owned()))
            .collect();
        let application_state_call_graph = application_state_call_graph(
            &runtime_singleton_bindings,
            &mut computation_db,
            &mut component_db,
            &mut constructible_db,
        );
        let codegen_types = codegen_types(&package_graph, &krate_collection);
        exit_on_errors!(diagnostics);
        Ok(Self {
            package_graph,
            handler_call_graphs,
            component_db,
            computation_db,
            application_state_call_graph,
            runtime_singleton_bindings,
            request_scoped_framework_bindings,
            codegen_types,
        })
    }

    /// Generate the manifest and the Rust code for the analysed application.
    ///
    /// They are generated in-memory, they are not persisted to disk.
    pub fn codegen(&self) -> Result<GeneratedApp, anyhow::Error> {
        let (cargo_toml, mut package_ids2deps) = codegen::codegen_manifest(
            &self.package_graph,
            &self.handler_call_graphs,
            &self.application_state_call_graph.call_graph,
            &self.request_scoped_framework_bindings,
            &self.codegen_types,
            &self.component_db,
            &self.computation_db,
        );
        let generated_app_package_id = PackageId::new(GENERATED_APP_PACKAGE_ID);
        let toolchain_package_ids = TOOLCHAIN_CRATES
            .iter()
            .map(|p| PackageId::new(*p))
            .collect::<Vec<_>>();
        for package_id in &toolchain_package_ids {
            package_ids2deps.insert(package_id.clone(), package_id.repr().into());
        }
        package_ids2deps.insert(generated_app_package_id, "crate".into());

        let lib_rs = codegen::codegen_app(
            &self.handler_call_graphs,
            &self.application_state_call_graph,
            &self.request_scoped_framework_bindings,
            &package_ids2deps,
            &self.runtime_singleton_bindings,
            &self.component_db,
            &self.computation_db,
        )?;
        Ok(GeneratedApp { lib_rs, cargo_toml })
    }

    /// A representation of an `App` geared towards debugging and testing.
    pub fn diagnostic_representation(&self) -> AppDiagnostics {
        let mut handler_graphs = IndexMap::new();
        let (_, mut package_ids2deps) = codegen::codegen_manifest(
            &self.package_graph,
            &self.handler_call_graphs,
            &self.application_state_call_graph.call_graph,
            &self.request_scoped_framework_bindings,
            &self.codegen_types,
            &self.component_db,
            &self.computation_db,
        );
        // TODO: dry this up in one place.
        let generated_app_package_id = PackageId::new(GENERATED_APP_PACKAGE_ID);
        let toolchain_package_ids = TOOLCHAIN_CRATES
            .iter()
            .map(|p| PackageId::new(*p))
            .collect::<Vec<_>>();
        for package_id in &toolchain_package_ids {
            package_ids2deps.insert(package_id.clone(), package_id.repr().into());
        }
        package_ids2deps.insert(generated_app_package_id, "crate".into());

        for (router_key, handler_call_graph) in &self.handler_call_graphs {
            let method = router_key
                .method_guard
                .as_ref()
                .map(|methods| {
                    methods
                        .iter()
                        .map(|method| method.to_string())
                        .collect::<Vec<_>>()
                        .join(" | ")
                })
                .unwrap_or("*".to_string());
            handler_graphs.insert(
                router_key.to_owned(),
                handler_call_graph
                    .dot(&package_ids2deps, &self.component_db, &self.computation_db)
                    .replace(
                        "digraph",
                        &format!("digraph \"{method} {}\"", router_key.path),
                    ),
            );
        }
        let application_state_graph = self
            .application_state_call_graph
            .call_graph
            .dot(&package_ids2deps, &self.component_db, &self.computation_db)
            .replace("digraph", "digraph app_state");
        AppDiagnostics {
            handlers: handler_graphs,
            application_state: application_state_graph,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum BuildError {
    #[error(transparent)]
    HandlerError(#[from] Box<CallableResolutionError>),
    #[error(transparent)]
    GenericError(#[from] anyhow::Error),
}

/// A representation of an `App` geared towards debugging and testing.
///
/// It contains the DOT representation of all the call graphs underpinning the originating `App`.
/// The DOT representation can be used for snapshot testing and/or troubleshooting.
pub struct AppDiagnostics {
    pub handlers: IndexMap<RouterKey, String>,
    pub application_state: String,
}

impl AppDiagnostics {
    /// Persist the diagnostic information to disk, using one file per handler within the specified
    /// directory.
    pub fn persist(&self, directory: &Path) -> Result<(), anyhow::Error> {
        let handler_directory = directory.join("handlers");
        fs_err::create_dir_all(&handler_directory)?;
        for (router_key, handler) in &self.handlers {
            let method = router_key
                .method_guard
                .as_ref()
                .map(|methods| {
                    methods
                        .iter()
                        .map(|method| method.to_string())
                        .collect::<Vec<_>>()
                        .join(" | ")
                })
                .unwrap_or("*".to_string());
            let path = router_key.path.trim_start_matches('/');
            let filepath = handler_directory.join(format!("{method} {path}.dot"));
            let mut file = fs_err::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(filepath)?;
            file.write_all(handler.as_bytes())?;
        }
        let mut file = fs_err::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(directory.join("app_state.dot"))?;
        file.write_all(self.application_state.as_bytes())?;
        Ok(())
    }

    /// Save all diagnostics in a single file instead of having one file per handler.
    pub fn persist_flat(&self, filepath: &Path) -> Result<(), anyhow::Error> {
        let file = fs_err::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(filepath)?;
        let mut file = BufWriter::new(file);

        for handler in self.handlers.values() {
            file.write_all(handler.as_bytes())?;
        }
        file.write_all(self.application_state.as_bytes())?;
        file.flush()?;
        Ok(())
    }
}

/// Determine the set of singleton types that are required to execute the constructors and handlers
/// registered by the application.
/// These singletons will be attached to the overall application state.
fn get_required_singleton_types<'a>(
    handler_call_graphs: impl Iterator<Item = (&'a RouterKey, &'a CallGraph)>,
    types_provided_by_the_framework: &BiHashMap<Ident, ResolvedType>,
    constructibles_db: &ConstructibleDb,
    component_db: &ComponentDb,
) -> IndexSet<(ResolvedType, ComponentId)> {
    let mut singletons_to_be_built = IndexSet::new();
    for (_, handler_call_graph) in handler_call_graphs {
        let root_component_id = handler_call_graph.root_component_id();
        let root_component_scope_id = component_db.scope_id(root_component_id);
        for required_input in handler_call_graph.required_input_types() {
            let required_input = if let ResolvedType::Reference(t) = &required_input {
                if !t.is_static {
                    // We can't store non-'static references in the application state, so we expect
                    // to see the referenced type in there.
                    t.inner.deref()
                } else {
                    &required_input
                }
            } else {
                &required_input
            };
            if !types_provided_by_the_framework.contains_right(required_input) {
                let component_id = constructibles_db
                    .get(
                        root_component_scope_id.clone(),
                        required_input,
                        component_db.user_component_db.scope_graph(),
                    )
                    .unwrap();
                assert_eq!(
                    component_db.lifecycle(component_id),
                    Some(&Lifecycle::Singleton)
                );
                singletons_to_be_built.insert((required_input.to_owned(), component_id));
            }
        }
    }
    singletons_to_be_built
}

/// Return the set of name bindings injected by `pavex` into the processing context for
/// an incoming request (e.g. the incoming request itself!).  
/// The types injected here can be used by constructors and handlers even though no constructor
/// has been explicitly registered for them by the developer.
fn framework_bindings(
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
) -> BiHashMap<Ident, ResolvedType> {
    let http_request = process_framework_path(
        "pavex_runtime::http::Request::<pavex_runtime::hyper::Body>",
        package_graph,
        krate_collection,
    );
    let raw_path_parameters = process_framework_path(
        "pavex_runtime::extract::route::RawRouteParams::<'server, 'request>",
        package_graph,
        krate_collection,
    );
    BiHashMap::from_iter(
        [
            (format_ident!("request"), http_request),
            (format_ident!("url_params"), raw_path_parameters),
        ]
        .into_iter(),
    )
}

/// Return the set of types that will be used in the generated code to build a functional
/// server scaffolding.  
fn codegen_types(
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
) -> HashSet<ResolvedType> {
    let error = process_framework_path("pavex_runtime::Error", package_graph, krate_collection);
    HashSet::from_iter([error])
}

/// Verify that all singletons needed at runtime implement `Send`, `Sync` and `Clone`.
/// This is required since `pavex` runs on a multi-threaded `tokio` runtime.
fn verify_singletons(
    runtime_singletons: &IndexSet<(ResolvedType, ComponentId)>,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
    diagnostics: &mut Vec<miette::Error>,
) {
    fn missing_trait_implementation(
        e: MissingTraitImplementationError,
        component_id: ComponentId,
        package_graph: &PackageGraph,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let HydratedComponent::Constructor(c) = component_db.hydrated_component(component_id, computation_db) else {
            unreachable!()
        };
        let component_id = match c.0 {
            Computation::Callable(_) => component_id,
            Computation::MatchResult(_) => component_db.fallible_id(component_id),
            Computation::BorrowSharedReference(_) => component_db.owned_id(component_id),
        };
        let user_component_id = component_db.user_component_id(component_id).unwrap();
        let user_component_db = &component_db.user_component_db;
        let user_component = &user_component_db[user_component_id];
        let component_kind = user_component.callable_type();
        let location = user_component_db.get_location(user_component_id);
        let source = match location.source_file(package_graph) {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(e.into());
                return;
            }
        };
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .map(|s| s.labeled(format!("The {component_kind} was registered here")));
        let help = "All singletons must implement the `Send`, `Sync` and `Clone` traits.\n \
                `pavex` runs on a multi-threaded HTTP server and singletons must be shared \
                 across all worker threads."
            .into();
        let diagnostic = CompilerDiagnostic::builder(source, e)
            .optional_label(label)
            .help(help)
            .build();
        diagnostics.push(diagnostic.into());
    }

    let send = process_framework_path("core::marker::Send", package_graph, krate_collection);
    let sync = process_framework_path("core::marker::Sync", package_graph, krate_collection);
    let clone = process_framework_path("core::clone::Clone", package_graph, krate_collection);
    for (singleton_type, component_id) in runtime_singletons {
        for trait_ in [&send, &sync, &clone] {
            let ResolvedType::ResolvedPath(trait_) = trait_ else { unreachable!() };
            if let Err(e) = assert_trait_is_implemented(krate_collection, singleton_type, trait_) {
                missing_trait_implementation(
                    e,
                    *component_id,
                    package_graph,
                    component_db,
                    computation_db,
                    diagnostics,
                );
            }
        }
    }
}
