use std::collections::BTreeSet;
use std::fmt::Debug;
use std::io::{BufWriter, Write};
use std::path::Path;

use ahash::{HashMap, HashMapExt};
use guppy::graph::PackageGraph;
use indexmap::IndexMap;

use pavex_bp_schema::Blueprint;

use crate::compiler::analyses::application_config::ApplicationConfig;
use crate::compiler::analyses::application_state::ApplicationState;
use crate::compiler::analyses::call_graph::{
    ApplicationStateCallGraph, application_state_call_graph,
};
use crate::compiler::analyses::cloning::{
    clonables_can_be_cloned, never_clones_should_not_be_copyable,
};
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::config_types::ConfigTypeDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::compiler::analyses::framework_items::FrameworkItemDb;
use crate::compiler::analyses::prebuilt_types::PrebuiltTypeDb;
use crate::compiler::analyses::processing_pipeline::RequestHandlerPipeline;
use crate::compiler::analyses::router::Router;
use crate::compiler::analyses::unused::detect_unused;
use crate::compiler::analyses::user_components::UserComponentDb;
use crate::compiler::generated_app::GeneratedApp;
use crate::compiler::resolvers::CallableResolutionError;
use crate::compiler::{codegen, path_parameters};
use crate::diagnostic::DiagnosticSink;
use crate::rustdoc::CrateCollection;

pub(crate) const GENERATED_APP_PACKAGE_ID: &str = "crate";

/// An in-memory representation that can be used to generate application code that matches
/// the constraints and instructions from a [`Blueprint`] instance.
pub struct App {
    package_graph: PackageGraph,
    router: Router,
    handler_id2pipeline: IndexMap<ComponentId, RequestHandlerPipeline>,
    application_state_call_graph: ApplicationStateCallGraph,
    framework_item_db: FrameworkItemDb,
    application_state: ApplicationState,
    application_config: ApplicationConfig,
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
        krate_collection: CrateCollection,
    ) -> Result<(Self, DiagnosticSink), DiagnosticSink> {
        /// Exit early if there is at least one error.
        macro_rules! exit_on_errors {
            ($var:ident) => {
                if !$var.is_empty()
                    && $var.diagnostics().iter().any(|e| {
                        let severity = e.severity();
                        severity == Some(miette::Severity::Error) || severity.is_none()
                    })
                {
                    return Err($var);
                }
            };
        }

        let package_graph = krate_collection.package_graph().to_owned();
        let mut diagnostics = DiagnosticSink::new(package_graph.clone());
        let mut computation_db = ComputationDb::new();
        let mut prebuilt_type_db = PrebuiltTypeDb::new();
        let mut config_type_db = ConfigTypeDb::new();
        let Ok((router, user_component_db)) = UserComponentDb::build(
            &bp,
            &mut computation_db,
            &mut prebuilt_type_db,
            &mut config_type_db,
            &krate_collection,
            &mut diagnostics,
        ) else {
            return Err(diagnostics);
        };

        let framework_item_db = FrameworkItemDb::new(&krate_collection);
        let mut component_db = ComponentDb::build(
            user_component_db,
            &framework_item_db,
            &mut computation_db,
            prebuilt_type_db,
            config_type_db,
            &package_graph,
            &krate_collection,
            &mut diagnostics,
        );
        let router = Router::lift(router, component_db.user_component_id2component_id());
        exit_on_errors!(diagnostics);
        let mut constructible_db = ConstructibleDb::build(
            &mut component_db,
            &mut computation_db,
            &krate_collection,
            &framework_item_db,
            &mut diagnostics,
        );
        clonables_can_be_cloned(
            &component_db,
            &computation_db,
            &krate_collection,
            &mut diagnostics,
        );
        never_clones_should_not_be_copyable(
            &component_db,
            &computation_db,
            &krate_collection,
            &mut diagnostics,
        );
        exit_on_errors!(diagnostics);
        let handler_id2pipeline = {
            let handler_ids: BTreeSet<_> = router.handler_ids();
            let route_infos = router.route_infos();
            let mut handler_pipelines = IndexMap::new();
            for (i, handler_id) in handler_ids.into_iter().enumerate() {
                let route_info = &route_infos[handler_id];
                let span = tracing::info_span!("Compute request processing pipeline", route_info = %route_info);
                let _guard = span.enter();
                let Ok(processing_pipeline) = RequestHandlerPipeline::new(
                    handler_id,
                    format!("route_{i}"),
                    &mut computation_db,
                    &mut component_db,
                    &mut constructible_db,
                    &framework_item_db,
                    &krate_collection,
                    &mut diagnostics,
                ) else {
                    continue;
                };
                handler_pipelines.insert(handler_id, processing_pipeline);
            }
            handler_pipelines
        };
        path_parameters::verify_path_parameters(
            &router,
            &handler_id2pipeline,
            &computation_db,
            &component_db,
            &krate_collection,
            &mut diagnostics,
        );
        let application_config =
            ApplicationConfig::new(&component_db, &computation_db, &mut diagnostics);
        exit_on_errors!(diagnostics);

        let application_state = ApplicationState::new(
            &handler_id2pipeline,
            &framework_item_db,
            &constructible_db,
            &component_db,
            &computation_db,
            &krate_collection,
            &mut diagnostics,
        );
        exit_on_errors!(diagnostics);

        let codegen_deps = codegen_deps(&package_graph);
        let Ok(application_state_call_graph) = application_state_call_graph(
            &application_state,
            &mut computation_db,
            &mut component_db,
            &mut constructible_db,
            &framework_item_db,
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
                application_state,
                application_config,
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
            &self.application_config,
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
            &self.application_state,
            &self.application_config,
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
            &self.application_config,
            &self.framework_item_db.bindings(),
            &self.codegen_deps,
            &self.component_db,
            &self.computation_db,
        );

        let infos = self.router.route_infos();
        let handlers = self
            .router
            .handler_ids()
            .into_iter()
            .map(|id| {
                let info = &infos[id];
                self.handler_id2pipeline[&id]
                    .graph_iter()
                    .enumerate()
                    .map(|(i, graph)| {
                        graph
                            .dot(&package_ids2deps, &self.component_db, &self.computation_db)
                            .replace("digraph", &format!("digraph \"{info} - {i}\""))
                    })
                    .collect()
            })
            .collect();
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
    pub handlers: Vec<Vec<String>>,
    pub application_state: String,
}

impl AppDiagnostics {
    /// Save all diagnostics in a single file.
    pub fn persist_flat(&self, filepath: &Path) -> Result<(), anyhow::Error> {
        let file = fs_err::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(filepath)?;
        let mut file = BufWriter::new(file);

        for handler_graphs in &self.handlers {
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
        .find(|p| p.name() == "thiserror" && p.version().major == 2)
        .expect("Expected to find `thiserror@2` in the package graph, but it was not there.")
        .id();
    let matchit = package_graph
        .packages()
        .find(|p| p.name() == "matchit" && p.version().major == 0 && p.version().minor == 8)
        .expect("Expected to find `matchit@0.8` in the package graph, but it was not there.")
        .id();
    let serde = package_graph
        .packages()
        .find(|p| p.name() == "serde" && p.version().major == 1)
        .expect("Expected to find `serde@1` in the package graph, but it was not there.")
        .id();

    name2id.insert("http".to_string(), http.clone());
    name2id.insert("hyper".to_string(), hyper.clone());
    name2id.insert("matchit".to_string(), matchit.clone());
    name2id.insert("pavex".to_string(), pavex.clone());
    name2id.insert("thiserror".to_string(), thiserror.clone());
    name2id.insert("serde".to_string(), serde.clone());
    name2id
}
