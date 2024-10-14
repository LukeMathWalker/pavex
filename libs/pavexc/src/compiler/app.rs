use std::collections::BTreeSet;
use std::fmt::Debug;
use std::io::{BufWriter, Write};
use std::ops::Deref;
use std::path::Path;

use ahash::{HashMap, HashMapExt};
use bimap::BiHashMap;
use guppy::graph::PackageGraph;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use proc_macro2::Ident;
use quote::format_ident;

use pavex_bp_schema::{Blueprint, Lifecycle};

use crate::compiler::analyses::call_graph::{
    application_state_call_graph, ApplicationStateCallGraph, RawCallGraphExt,
};
use crate::compiler::analyses::cloning::clonables_can_be_cloned;
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::compiler::analyses::framework_items::FrameworkItemDb;
use crate::compiler::analyses::prebuilt_types::PrebuiltTypeDb;
use crate::compiler::analyses::processing_pipeline::RequestHandlerPipeline;
use crate::compiler::analyses::router::Router;
use crate::compiler::analyses::singletons::{
    runtime_singletons_are_thread_safe, runtime_singletons_can_be_cloned_if_needed,
};
use crate::compiler::analyses::unused::detect_unused;
use crate::compiler::analyses::user_components::UserComponentDb;
use crate::compiler::generated_app::GeneratedApp;
use crate::compiler::resolvers::CallableResolutionError;
use crate::compiler::{codegen, path_parameter_validation};
use crate::language::ResolvedType;
use crate::rustdoc::CrateCollection;
use crate::utils::anyhow2miette;

pub(crate) const GENERATED_APP_PACKAGE_ID: &str = "crate";

/// An in-memory representation that can be used to generate application code that matches
/// the constraints and instructions from a [`Blueprint`] instance.
pub struct App {
    package_graph: PackageGraph,
    router: Router,
    handler_id2pipeline: IndexMap<ComponentId, RequestHandlerPipeline>,
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
    pub fn build(
        bp: Blueprint,
        docs_toolchain_name: String,
    ) -> Result<(Self, Vec<miette::Error>), Vec<miette::Error>> {
        /// Exit early if there is at least one error.
        macro_rules! exit_on_errors {
            ($var:ident) => {
                if !$var.is_empty()
                    && $var.iter().any(|e| {
                        let severity = e.severity();
                        severity == Some(miette::Severity::Error) || severity.is_none()
                    })
                {
                    return Err($var);
                }
            };
        }

        let krate_collection = CrateCollection::new(
            docs_toolchain_name,
            std::env::current_dir().expect("Failed to determine the current directory"),
        )
        .map_err(|e| vec![anyhow2miette(e)])?;
        let package_graph = krate_collection.package_graph().to_owned();
        let mut diagnostics = vec![];
        let mut computation_db = ComputationDb::new();
        let mut prebuilt_type_db = PrebuiltTypeDb::new();
        let Ok((router, user_component_db)) = UserComponentDb::build(
            &bp,
            &mut computation_db,
            &mut prebuilt_type_db,
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
            prebuilt_type_db,
            &package_graph,
            &krate_collection,
            &mut diagnostics,
        );
        let router = Router::lift(router, component_db.user_component_id2component_id());
        exit_on_errors!(diagnostics);
        let mut constructible_db = ConstructibleDb::build(
            &mut component_db,
            &mut computation_db,
            &package_graph,
            &krate_collection,
            &framework_item_db,
            &mut diagnostics,
        );
        clonables_can_be_cloned(
            &component_db,
            &computation_db,
            &package_graph,
            &krate_collection,
            &mut diagnostics,
        );
        exit_on_errors!(diagnostics);
        let handler_id2pipeline = {
            let handler_ids: BTreeSet<_> = router
                .route_path2sub_router
                .values()
                .flat_map(|leaf_router| leaf_router.handler_ids())
                .chain(std::iter::once(&router.root_fallback_id))
                .collect();
            let mut handler_pipelines = IndexMap::new();
            for (i, handler_id) in handler_ids.into_iter().enumerate() {
                let route_info = &router.handler_id2route_info[handler_id];
                let span = tracing::info_span!("Compute request processing pipeline", route_info = %route_info);
                let _guard = span.enter();
                let Ok(processing_pipeline) = RequestHandlerPipeline::new(
                    *handler_id,
                    format!("route_{i}"),
                    &mut computation_db,
                    &mut component_db,
                    &mut constructible_db,
                    &framework_item_db,
                    &package_graph,
                    &krate_collection,
                    &mut diagnostics,
                ) else {
                    continue;
                };
                handler_pipelines.insert(*handler_id, processing_pipeline);
            }
            handler_pipelines
        };
        path_parameter_validation::verify_path_parameters(
            &router,
            &handler_id2pipeline,
            &computation_db,
            &component_db,
            &package_graph,
            &krate_collection,
            &mut diagnostics,
        );
        exit_on_errors!(diagnostics);

        let runtime_singletons: IndexSet<(ResolvedType, ComponentId)> =
            get_required_singleton_types(
                handler_id2pipeline.values(),
                &framework_item_db,
                &constructible_db,
                &component_db,
            );

        runtime_singletons_are_thread_safe(
            &runtime_singletons,
            &component_db,
            &computation_db,
            &package_graph,
            &krate_collection,
            &mut diagnostics,
        );
        runtime_singletons_can_be_cloned_if_needed(
            handler_id2pipeline.values(),
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
            &framework_item_db,
            &package_graph,
            &krate_collection,
            &mut diagnostics,
        ) else {
            return Err(diagnostics);
        };
        detect_unused(
            handler_id2pipeline.values(),
            &application_state_call_graph,
            &component_db,
            &computation_db,
            &package_graph,
            &mut diagnostics,
        );
        exit_on_errors!(diagnostics);
        Ok((
            Self {
                package_graph,
                router,
                handler_id2pipeline,
                component_db,
                computation_db,
                application_state_call_graph,
                framework_item_db,
                runtime_singleton_bindings,
                codegen_deps,
            },
            diagnostics,
        ))
    }

    /// Generate the manifest and the Rust code for the analysed application.
    ///
    /// They are generated in-memory, they are not persisted to disk.
    #[tracing::instrument(skip_all, level = tracing::Level::INFO)]
    pub fn codegen(&self) -> Result<GeneratedApp, anyhow::Error> {
        let framework_bindings = self.framework_item_db.bindings();
        let (cargo_toml, package_ids2deps) = codegen::codegen_manifest(
            &self.package_graph,
            self.handler_id2pipeline.values(),
            &self.application_state_call_graph.call_graph.call_graph,
            &framework_bindings,
            &self.codegen_deps,
            &self.component_db,
            &self.computation_db,
        );
        let lib_rs = codegen::codegen_app(
            &self.router,
            &self.handler_id2pipeline,
            &self.application_state_call_graph,
            &framework_bindings,
            &package_ids2deps,
            &self.runtime_singleton_bindings,
            &self.codegen_deps,
            &self.component_db,
            &self.computation_db,
            &self.framework_item_db,
        )?;
        Ok(GeneratedApp {
            lib_rs,
            cargo_toml,
            package_graph: self.package_graph.clone(),
        })
    }

    /// A representation of an `App` geared towards debugging and testing.
    pub fn diagnostic_representation(&self) -> AppDiagnostics {
        let (_, package_ids2deps) = codegen::codegen_manifest(
            &self.package_graph,
            self.handler_id2pipeline.values(),
            &self.application_state_call_graph.call_graph.call_graph,
            &self.framework_item_db.bindings(),
            &self.codegen_deps,
            &self.component_db,
            &self.computation_db,
        );

        let mut handlers = IndexMap::new();
        for (path, method_router) in &self.router.route_path2sub_router {
            for (handler_id, methods) in method_router
                .handler_id2methods
                .iter()
                .map(|(k, v)| (*k, Some(v)))
                .chain(std::iter::once((method_router.fallback_id, None)))
            {
                let method = methods
                    .map(|m| m.iter().join(" | "))
                    .unwrap_or_else(|| "*".into());
                let pipeline = &self.handler_id2pipeline[&handler_id];
                let mut handler_graphs = Vec::new();
                for (i, graph) in pipeline.graph_iter().enumerate() {
                    handler_graphs.push(
                        graph
                            .dot(&package_ids2deps, &self.component_db, &self.computation_db)
                            .replace("digraph", &format!("digraph \"{method} {} - {i}\"", path)),
                    );
                }
                handlers.insert((path.to_owned(), method), handler_graphs);
            }
        }
        let application_state_graph = self
            .application_state_call_graph
            .call_graph
            .dot(&package_ids2deps, &self.component_db, &self.computation_db)
            .replace("digraph", "digraph app_state");
        AppDiagnostics {
            handlers,
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
    /// For each handler, we have a sequence of DOT graphs representing the call graph of each
    /// middleware in their pipeline and the request handler itself.
    ///
    /// The key is a tuple of `(path, methods)`, where `path` is the path of the route and `methods`
    /// is the concatenation of all the HTTP methods that the handler can handle.
    pub handlers: IndexMap<(String, String), Vec<String>>,
    pub application_state: String,
}

impl AppDiagnostics {
    /// Persist the diagnostic information to disk, using one file per handler within the specified
    /// directory.
    pub fn persist(&self, directory: &Path) -> Result<(), anyhow::Error> {
        let handler_directory = directory.join("handlers");
        fs_err::create_dir_all(&handler_directory)?;
        for ((path, method), handler_graphs) in &self.handlers {
            let path = path.trim_start_matches('/');
            let filepath = handler_directory.join(format!("{method} {path}.dot"));
            let mut file = fs_err::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(filepath)?;
            for handler_graph in handler_graphs {
                file.write_all(handler_graph.as_bytes())?;
                // Add a newline between graphs for readability
                file.write_all("\n".as_bytes())?;
            }
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

        for handler_graphs in self.handlers.values() {
            for handler_graph in handler_graphs {
                file.write_all(handler_graph.as_bytes())?;
                // Add a newline between graphs for readability
                file.write_all("\n".as_bytes())?;
            }
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
    handler_pipelines: impl Iterator<Item = &'a RequestHandlerPipeline>,
    framework_item_db: &FrameworkItemDb,
    constructibles_db: &ConstructibleDb,
    component_db: &ComponentDb,
) -> IndexSet<(ResolvedType, ComponentId)> {
    let mut singletons_to_be_built = IndexSet::new();
    for handler_pipeline in handler_pipelines {
        for graph in handler_pipeline.graph_iter() {
            let root_component_id = graph.root_component_id();
            let root_component_scope_id = component_db.scope_id(root_component_id);
            for required_input in graph.call_graph.required_input_types() {
                let required_input = if let ResolvedType::Reference(t) = &required_input {
                    if !t.lifetime.is_static() {
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
                // This can be `None` if the required input is a singleton that is used
                // by a downstream stage of the processing pipeline.
                // The singleton will be passed down using `Next` pass-along state from
                // the very first stage in the pipeline all the way to the stage that needs it,
                // but the singleton constructor might not be visible in the scope of the current
                // stage of the processing pipeline.
                if let Some((component_id, _)) = constructibles_db.get(
                    root_component_scope_id,
                    required_input,
                    component_db.scope_graph(),
                ) {
                    let lifecycle = component_db.lifecycle(component_id);
                    #[cfg(debug_assertions)]
                    {
                        // No scenario where this should/could happen.
                        assert_ne!(lifecycle, Lifecycle::Transient);
                    }

                    // Some inputs are request-scoped because they come from the `Next<_>` pass-along
                    // state. We don't care about those here.
                    if lifecycle == Lifecycle::Singleton {
                        singletons_to_be_built.insert((required_input.to_owned(), component_id));
                    }
                }
            }
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
        .find(|p| p.name() == "http" && p.version().major == 1)
        .expect("Expected to find `http@1` in the package graph, but it was not there.")
        .id();
    let hyper = package_graph
        .packages()
        .find(|p| p.name() == "hyper" && p.version().major == 1)
        .expect("Expected to find `hyper@1` in the package graph, but it was not there.")
        .id();
    let thiserror = package_graph
        .packages()
        .find(|p| p.name() == "thiserror" && p.version().major == 1)
        .expect("Expected to find `thiserror@1` in the package graph, but it was not there.")
        .id();
    let matchit = package_graph
        .packages()
        .find(|p| p.name() == "pavex_matchit" && p.version().major == 0 && p.version().minor == 7)
        .expect("Expected to find `pavex_matchit@0.7` in the package graph, but it was not there.")
        .id();

    name2id.insert("http".to_string(), http.clone());
    name2id.insert("hyper".to_string(), hyper.clone());
    name2id.insert("matchit".to_string(), matchit.clone());
    name2id.insert("pavex".to_string(), pavex.clone());
    name2id.insert("thiserror".to_string(), thiserror.clone());
    name2id
}
