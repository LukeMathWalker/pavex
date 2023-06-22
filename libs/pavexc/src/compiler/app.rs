use std::fmt::Debug;
use std::io::{BufWriter, Write};
use std::ops::Deref;
use std::path::Path;

use ahash::{HashMap, HashMapExt};
use bimap::BiHashMap;
use guppy::graph::PackageGraph;
use indexmap::{IndexMap, IndexSet};
use miette::miette;
use proc_macro2::Ident;
use quote::format_ident;

use pavex::blueprint::{constructor::Lifecycle, Blueprint};

use crate::compiler::analyses::call_graph::{
    application_state_call_graph, handler_call_graph, ApplicationStateCallGraph, OrderedCallGraph,
    RawCallGraphExt,
};
use crate::compiler::analyses::components::{ComponentDb, ComponentId, HydratedComponent};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::compiler::analyses::framework_items::FrameworkItemDb;
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
use crate::rustdoc::CrateCollection;

pub(crate) const GENERATED_APP_PACKAGE_ID: &str = "crate";

/// An in-memory representation that can be used to generate application code that matches
/// the constraints and instructions from a [`Blueprint`] instance.
pub struct App {
    package_graph: PackageGraph,
    handler_call_graphs: IndexMap<RouterKey, OrderedCallGraph>,
    application_state_call_graph: ApplicationStateCallGraph,
    framework_item_db: FrameworkItemDb,
    runtime_singleton_bindings: BiHashMap<Ident, ResolvedType>,
    codegen_deps: HashMap<String, guppy::PackageId>,
    component_db: ComponentDb,
    computation_db: ComputationDb,
}

impl App {
    #[tracing::instrument(skip_all)]
    /// Process the [`Blueprint`] created by user into an [`App`] instance—an in-memory
    /// representation that can be used to generate application code that matches the constraints
    /// and instructions in the blueprint.
    ///
    /// Many different things can go wrong during this process: this method tries its best to
    /// report all errors to the user, but it may not be able to do so in all cases.
    pub fn build(bp: Blueprint, project_fingerprint: String) -> Result<Self, Vec<miette::Error>> {
        /// Exit early if there is at least one error.
        macro_rules! exit_on_errors {
            ($var:ident) => {
                if !$var.is_empty() {
                    return Err($var);
                }
            };
        }

        let krate_collection =
            CrateCollection::new(project_fingerprint).map_err(|e| vec![miette!(e)])?;
        let package_graph = krate_collection.package_graph().to_owned();
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
        let framework_item_db = FrameworkItemDb::new(&package_graph, &krate_collection);
        let mut component_db = ComponentDb::build(
            user_component_db,
            &framework_item_db,
            &mut computation_db,
            &package_graph,
            &krate_collection,
            &mut diagnostics,
        );
        exit_on_errors!(diagnostics);
        let mut constructible_db = ConstructibleDb::build(
            &mut component_db,
            &mut computation_db,
            &package_graph,
            &krate_collection,
            &framework_item_db,
            &mut diagnostics,
        );
        exit_on_errors!(diagnostics);
        let handler_call_graphs = {
            let router = component_db.router().clone();
            let mut handler_call_graphs = IndexMap::with_capacity(router.len());
            for (router_key, handler_id) in router {
                let Ok(call_graph) = handler_call_graph(
                    handler_id,
                    &mut computation_db,
                    &mut component_db,
                    &constructible_db,
                    &package_graph,
                    &krate_collection,
                    &mut diagnostics,
                ) else {
                    continue;
                };
                handler_call_graphs.insert(router_key.to_owned(), call_graph);
            }
            handler_call_graphs
        };
        route_parameter_validation::verify_route_parameters(
            handler_call_graphs
                .iter()
                .map(|(k, ocg)| (k, &ocg.call_graph)),
            &computation_db,
            &component_db,
            &package_graph,
            &krate_collection,
            &mut diagnostics,
        );
        exit_on_errors!(diagnostics);

        let runtime_singletons: IndexSet<(ResolvedType, ComponentId)> =
            get_required_singleton_types(
                handler_call_graphs.iter(),
                &framework_item_db,
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
        let codegen_deps = codegen_deps(&package_graph);
        let Ok(application_state_call_graph) = application_state_call_graph(
            &runtime_singleton_bindings,
            &mut computation_db,
            &mut component_db,
            &mut constructible_db,
            &package_graph,
            &krate_collection,
            &mut diagnostics,
        ) else {
            return Err(diagnostics);
        };
        exit_on_errors!(diagnostics);
        Ok(Self {
            package_graph,
            handler_call_graphs,
            component_db,
            computation_db,
            application_state_call_graph,
            framework_item_db,
            runtime_singleton_bindings,
            codegen_deps,
        })
    }

    /// Generate the manifest and the Rust code for the analysed application.
    ///
    /// They are generated in-memory, they are not persisted to disk.
    #[tracing::instrument(skip_all, level=tracing::Level::INFO)]
    pub fn codegen(&self) -> Result<GeneratedApp, anyhow::Error> {
        let framework_bindings = self.framework_item_db.bindings();
        let (cargo_toml, package_ids2deps) = codegen::codegen_manifest(
            &self.package_graph,
            self.handler_call_graphs.values().map(|cg| &cg.call_graph),
            &self.application_state_call_graph.call_graph.call_graph,
            &framework_bindings,
            &self.codegen_deps,
            &self.component_db,
            &self.computation_db,
        );
        let lib_rs = codegen::codegen_app(
            &self.handler_call_graphs,
            &self.application_state_call_graph,
            &framework_bindings,
            &package_ids2deps,
            &self.runtime_singleton_bindings,
            &self.codegen_deps,
            &self.component_db,
            &self.computation_db,
        )?;
        Ok(GeneratedApp {
            lib_rs,
            cargo_toml,
            package_graph: self.package_graph.clone(),
        })
    }

    /// A representation of an `App` geared towards debugging and testing.
    pub fn diagnostic_representation(&self) -> AppDiagnostics {
        let mut handler_graphs = IndexMap::new();
        let (_, package_ids2deps) = codegen::codegen_manifest(
            &self.package_graph,
            self.handler_call_graphs.values().map(|c| &c.call_graph),
            &self.application_state_call_graph.call_graph.call_graph,
            &self.framework_item_db.bindings(),
            &self.codegen_deps,
            &self.component_db,
            &self.computation_db,
        );

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
    handler_call_graphs: impl Iterator<Item = (&'a RouterKey, &'a OrderedCallGraph)>,
    framework_item_db: &FrameworkItemDb,
    constructibles_db: &ConstructibleDb,
    component_db: &ComponentDb,
) -> IndexSet<(ResolvedType, ComponentId)> {
    let mut singletons_to_be_built = IndexSet::new();
    for (_, handler_call_graph) in handler_call_graphs {
        let root_component_id = handler_call_graph.root_component_id();
        let root_component_scope_id = component_db.scope_id(root_component_id);
        for required_input in handler_call_graph.call_graph.required_input_types() {
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
            // If it's a framework built-in, nothing to do!
            if framework_item_db.get_id(required_input).is_some() {
                continue;
            }
            let (component_id, _) = constructibles_db
                .get(
                    root_component_scope_id,
                    required_input,
                    component_db.scope_graph(),
                )
                .unwrap();
            assert_eq!(
                component_db.lifecycle(component_id),
                Some(&Lifecycle::Singleton)
            );
            singletons_to_be_built.insert((required_input.to_owned(), component_id));
        }
    }
    singletons_to_be_built
}

/// Return the set of dependencies that must be used directly by the generated code to build the
/// server scaffolding.
///
/// These lookups should never fail because `anyhow` and `http` are direct dependencies
/// of `pavex`, which must have been used in the first place to build the `Blueprint`.
fn codegen_deps(package_graph: &PackageGraph) -> HashMap<String, guppy::PackageId> {
    let mut name2id = HashMap::new();

    let pavex = package_graph
        .packages()
        .find(|p| p.name() == "pavex" && p.version().major == 0 && p.version().minor == 1)
        // TODO: Return a user diagnostic in case of a version mismatch between the
        //  CLI and the dependencies of the project (i.e. the `pavex` and `pavex_cli`
        //  versions).
        .expect("Expected to find `pavex@0.1` in the package graph, but it was not there.")
        .id();
    let http = package_graph
        .packages()
        .find(|p| p.name() == "http" && p.version().major == 0 && p.version().minor == 2)
        .expect("Expected to find `http@0.2` in the package graph, but it was not there.")
        .id();
    let hyper = package_graph
        .packages()
        .find(|p| p.name() == "hyper" && p.version().major == 0 && p.version().minor == 14)
        .expect("Expected to find `hyper@0.14` in the package graph, but it was not there.")
        .id();
    let thiserror = package_graph
        .packages()
        .find(|p| p.name() == "thiserror" && p.version().major == 1)
        .expect("Expected to find `thiserror@1` in the package graph, but it was not there.")
        .id();

    name2id.insert("http".to_string(), http.clone());
    name2id.insert("pavex".to_string(), pavex.clone());
    name2id.insert("hyper".to_string(), hyper.clone());
    name2id.insert("thiserror".to_string(), thiserror.clone());
    name2id
}

/// Verify that all singletons needed at runtime implement `Send`, `Sync` and `Clone`.
/// This is required since Pavex runs on a multi-threaded `tokio` runtime.
#[tracing::instrument(name = "Verify trait implementations for singletons", skip_all)]
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
            Computation::FrameworkItem(_) => unreachable!(),
        };
        let user_component_id = component_db.user_component_id(component_id).unwrap();
        let user_component_db = &component_db.user_component_db();
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
                Pavex runs on a multi-threaded HTTP server and singletons must be shared \
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
