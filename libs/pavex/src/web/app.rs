use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Debug;
use std::io::{BufWriter, Write};
use std::path::Path;

use bimap::BiHashMap;
use guppy::graph::PackageGraph;
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};
use miette::miette;
use proc_macro2::Ident;
use quote::format_ident;

use pavex_builder::AppBlueprint;
use pavex_builder::Lifecycle;
use pavex_builder::RawCallableIdentifiers;

use crate::diagnostic;
use crate::diagnostic::{get_registration_location, CompilerDiagnostic};
use crate::diagnostic::{
    get_registration_location_for_a_request_handler, LocationExt, SourceSpanExt,
};
use crate::language::{Callable, ResolvedPathSegment, ResolvedType};
use crate::language::{ResolvedPath, ResolvedPathQualifiedSelf};
use crate::rustdoc::CrateCollection;
use crate::rustdoc::TOOLCHAIN_CRATES;
use crate::web::call_graph::{application_state_call_graph, handler_call_graph};
use crate::web::call_graph::{ApplicationStateCallGraph, CallGraph};
use crate::web::constructors::Constructor;
use crate::web::error_handlers::ErrorHandler;
use crate::web::generated_app::GeneratedApp;
use crate::web::resolvers::{
    resolve_callable, resolve_type_path, CallableResolutionError, CallableType,
};
use crate::web::traits::{assert_trait_is_implemented, MissingTraitImplementationError};
use crate::web::{codegen, resolvers, utils};

pub(crate) const GENERATED_APP_PACKAGE_ID: &str = "crate";

pub struct App {
    package_graph: PackageGraph,
    router: BTreeMap<String, Callable>,
    handler_call_graphs: IndexMap<ResolvedPath, CallGraph>,
    application_state_call_graph: ApplicationStateCallGraph,
    runtime_singleton_bindings: BiHashMap<Ident, ResolvedType>,
    request_scoped_framework_bindings: BiHashMap<Ident, ResolvedType>,
    codegen_types: HashSet<ResolvedType>,
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

/// Exit early if there is at least one error.
macro_rules! exit_on_errors {
    ($var:ident) => {
        if !$var.is_empty() {
            return Err($var);
        }
    };
}

impl App {
    #[tracing::instrument(skip_all)]
    pub fn build(app_blueprint: AppBlueprint) -> Result<Self, Vec<miette::Error>> {
        // We collect all the unique raw identifiers from the blueprint.
        let raw_identifiers_db: HashSet<RawCallableIdentifiers> = {
            let mut set = HashSet::with_capacity(
                app_blueprint.request_handlers.len()
                    + app_blueprint.constructors.len()
                    + app_blueprint.constructor_error_handlers.len(),
            );
            set.extend(app_blueprint.request_handlers.iter().cloned());
            set.extend(app_blueprint.constructors.iter().cloned());
            set.extend(app_blueprint.constructor_error_handlers.values().cloned());
            set
        };

        let package_graph = compute_package_graph().map_err(|e| vec![e])?;

        let mut diagnostics = vec![];
        let mut krate_collection = CrateCollection::new(package_graph.clone());

        let resolved_paths2identifiers: HashMap<ResolvedPath, HashSet<RawCallableIdentifiers>> = {
            let mut map: HashMap<ResolvedPath, HashSet<RawCallableIdentifiers>> = HashMap::new();
            for raw_identifier in &raw_identifiers_db {
                match ResolvedPath::parse(raw_identifier, &package_graph) {
                    Ok(p) => {
                        map.entry(p).or_default().insert(raw_identifier.to_owned());
                    }
                    Err(e) => {
                        let diagnostic = e
                            .into_diagnostic(&app_blueprint, &package_graph)
                            .to_miette();
                        diagnostics.push(diagnostic);
                    }
                }
            }
            map
        };

        exit_on_errors!(diagnostics);

        // Important: there might be multiple identifiers pointing to the same callable path.
        // This is not necessarily an issue - e.g. the user might have registered the same
        // function as the handler for multiple routes.
        let identifiers2path = {
            let mut map: HashMap<RawCallableIdentifiers, ResolvedPath> =
                HashMap::with_capacity(raw_identifiers_db.len());
            for (path, identifiers) in &resolved_paths2identifiers {
                for identifier in identifiers {
                    map.insert(identifier.to_owned(), path.to_owned());
                }
            }
            map
        };

        let constructor_paths: IndexSet<ResolvedPath> = app_blueprint
            .constructors
            .iter()
            .map(|id| identifiers2path[id].clone())
            .collect();

        let request_handler_paths: IndexSet<ResolvedPath> = app_blueprint
            .request_handlers
            .iter()
            .map(|id| identifiers2path[id].clone())
            .collect();

        let error_handler_paths: IndexSet<ResolvedPath> = app_blueprint
            .constructor_error_handlers
            .iter()
            .map(|(_, error_handler_id)| identifiers2path[error_handler_id].clone())
            .collect();

        let constructor_paths2ids = {
            let mut set = BiHashMap::with_capacity(app_blueprint.constructors.len());
            for constructor_identifiers in &app_blueprint.constructors {
                let constructor_path = identifiers2path[constructor_identifiers].clone();
                set.insert(constructor_path, constructor_identifiers.to_owned());
            }
            set
        };

        let (constructor_callable_resolver, errors) =
            resolvers::resolve_constructors(&constructor_paths, &mut krate_collection);
        for e in errors {
            diagnostics.push(
                e.into_diagnostic(
                    &resolved_paths2identifiers,
                    |identifiers| app_blueprint.constructor_locations[identifiers].clone(),
                    &package_graph,
                    CallableType::Constructor,
                )
                .to_miette(),
            );
        }

        let mut constructors: IndexMap<ResolvedType, Constructor> = IndexMap::new();
        for callable in constructor_callable_resolver.right_values() {
            let constructor: Constructor = match callable.to_owned().try_into() {
                Ok(c) => c,
                Err(e) => {
                    diagnostics.push(
                        e.into_diagnostic(
                            &resolved_paths2identifiers,
                            &app_blueprint,
                            &package_graph,
                        )
                        .to_miette(),
                    );
                    continue;
                }
            };
            constructors.insert(constructor.output_type().to_owned(), constructor);
        }

        for (output_type, constructor) in &constructors {
            if let Constructor::Callable(callable) = constructor {
                if !utils::is_result(&output_type) {
                    let constructor_path = constructor_callable_resolver
                        .get_by_right(&callable)
                        .unwrap();
                    let constructor_id =
                        constructor_paths2ids.get_by_left(constructor_path).unwrap();
                    if let Some(error_handler_id) =
                        app_blueprint.constructor_error_handlers.get(constructor_id)
                    {
                        let diagnostic = ErrorHandlerForInfallibleConstructor {
                            error_handler_id: error_handler_id.to_owned(),
                        }
                        .into_diagnostic(&app_blueprint, &package_graph)
                        .to_miette();
                        diagnostics.push(diagnostic);
                    }
                }
            }
        }

        let (error_handler_path2callable, error_handler_callable2paths, errors) =
            resolvers::resolve_error_handlers(&error_handler_paths, &mut krate_collection);
        for e in errors {
            diagnostics.push(
                e.into_diagnostic(
                    &resolved_paths2identifiers,
                    |identifiers| app_blueprint.error_handler_locations[identifiers].clone(),
                    &package_graph,
                    CallableType::ErrorHandler,
                )
                .to_miette(),
            );
        }

        let (handler_path2callable, handler_callable2paths, handlers, errors) =
            resolvers::resolve_request_handlers(&request_handler_paths, &mut krate_collection);
        for e in errors {
            diagnostics.push(
                e.into_diagnostic(
                    &resolved_paths2identifiers,
                    |identifiers| {
                        get_registration_location_for_a_request_handler(&app_blueprint, identifiers)
                            .unwrap()
                            .to_owned()
                    },
                    &package_graph,
                    CallableType::RequestHandler,
                )
                .to_miette(),
            );
        }

        exit_on_errors!(diagnostics);

        // TODO: check if we have a constructor that returns T and another constructor returning
        //  &T

        // For each non-reference type, register an inlineable constructor that transforms
        // `T` in `&T`.
        let constructible_types: Vec<_> = constructors.keys().map(|t| t.to_owned()).collect();
        for t in constructible_types {
            if !t.is_shared_reference {
                let c = Constructor::shared_borrow(t.to_owned());
                constructors.insert(c.output_type().to_owned(), c);
            }
        }

        // TODO: check if we have a constructor that returns T and one/more constructors returning
        //  Result<T, _>

        // For each Result type, register a match constructor that transforms
        // `Result<T,E>` into `T` or `E`.
        let constructible_types: Vec<_> = constructors.keys().map(|t| t.to_owned()).collect();
        for t in constructible_types {
            if t.is_shared_reference {
                continue;
            }
            if utils::is_result(&t) {
                let m = Constructor::match_result(&t);
                constructors.insert(m.ok.output_type().to_owned(), m.ok);
                constructors.insert(m.err.output_type().to_owned(), m.err);
            }
        }

        let response_transformers = {
            let mut response_transformers = HashMap::<ResolvedType, Callable>::new();
            let into_response = process_framework_path(
                "pavex_runtime::response::IntoResponse",
                &package_graph,
                &krate_collection,
            );
            let into_response_path = into_response.resolved_path();
            for (callable, callable_type) in handlers
                .iter()
                .map(|h| (h, CallableType::RequestHandler))
                .chain(
                    error_handler_path2callable
                        .values()
                        .map(|e| (e, CallableType::ErrorHandler)),
                )
            {
                if let Some(output) = &callable.output {
                    // TODO: only the Ok variant must implement IntoResponse if output is a result
                    if response_transformers.get(output).is_some() {
                        // We already processed this type
                        continue;
                    }
                    // Verify that the output type implements the `IntoResponse` trait.
                    if let Err(e) =
                        assert_trait_is_implemented(&krate_collection, output, &into_response)
                    {
                        let diagnostic = OutputCannotBeConvertedIntoAResponse {
                            identifiers: (),
                            output_type: output.to_owned(),
                            callable_type,
                            source: e,
                        }
                        .into_diagnostic(&app_blueprint, &package_graph)
                        .to_miette();
                        diagnostics.push(diagnostic);
                        continue;
                    }
                    let output_path = output.resolved_path();
                    let mut transformer_segments = into_response_path.segments.clone();
                    transformer_segments.push(ResolvedPathSegment {
                        ident: "into_response".into(),
                        generic_arguments: vec![],
                    });
                    let transformer_path = ResolvedPath {
                        segments: transformer_segments,
                        qualified_self: Some(ResolvedPathQualifiedSelf {
                            position: into_response_path.segments.len() - 1,
                            path: Box::new(output_path),
                        }),
                        package_id: into_response_path.package_id.clone(),
                    };
                    let transformer =
                        // TODO: remove unwrap
                        resolve_callable(&krate_collection, &transformer_path).unwrap();
                    response_transformers.insert(output.to_owned(), transformer);
                }
            }
            response_transformers
        };

        exit_on_errors!(diagnostics);

        let mut router = BTreeMap::new();
        for (route, callable_identifiers) in app_blueprint.router {
            let callable_path = &identifiers2path[&callable_identifiers];
            router.insert(route, handler_path2callable[callable_path].to_owned());
        }

        let component2lifecycle = {
            let mut map = HashMap::<ResolvedType, Lifecycle>::new();
            for (output_type, constructor) in &constructors {
                let lifecycle = match constructor {
                    // The lifecycle of a references matches the lifecycle of the type
                    // it refers to.
                    Constructor::BorrowSharedReference(s) => map[&s.input].clone(),
                    // The lifecycle of the "unwrapped" type matches the lifecycle of the
                    // original `Result`
                    Constructor::MatchResult(m) => map[&m.input].clone(),
                    Constructor::Callable(c) => {
                        let callable_path = constructor_callable_resolver.get_by_right(c).unwrap();
                        let raw_identifiers =
                            constructor_paths2ids.get_by_left(callable_path).unwrap();
                        app_blueprint.component_lifecycles[raw_identifiers].clone()
                    }
                };
                map.insert(output_type.to_owned(), lifecycle);
            }
            map
        };

        exit_on_errors!(diagnostics);

        let constructor2error_handler: HashMap<Constructor, ErrorHandler> = {
            let mut map = HashMap::new();
            for (output_type, constructor) in &constructors {
                if let Constructor::Callable(callable) = constructor {
                    // Errors when building a singleton are failures to build the application state.
                    // They are not handled - they are just exposed to the caller in an ad-hoc
                    // error enumeration.
                    if let Some(Lifecycle::Singleton) = component2lifecycle.get(output_type) {
                        continue;
                    }
                    if utils::is_result(&output_type) {
                        let constructor_path = constructor_callable_resolver
                            .get_by_right(&callable)
                            .unwrap();
                        let constructor_id =
                            constructor_paths2ids.get_by_left(constructor_path).unwrap();
                        let error_handler_id =
                            &app_blueprint.constructor_error_handlers[constructor_id];
                        let error_handler_path = &identifiers2path[error_handler_id];
                        let error_handler = error_handler_path2callable
                            .get(error_handler_path)
                            // TODO: return an error asking for an error handler to be registered
                            .unwrap()
                            .to_owned();
                        // TODO: handle the validation error
                        let error_handler = ErrorHandler::new(error_handler, callable)
                            .expect("Failed to validate the error handler");
                        map.insert(constructor.to_owned(), error_handler);
                    }
                }
            }
            map
        };

        let mut handler_call_graphs = IndexMap::with_capacity(handlers.len());
        for callable in &handlers {
            handler_call_graphs.insert(
                callable.path.clone(),
                handler_call_graph(
                    callable.to_owned(),
                    &component2lifecycle,
                    &constructors,
                    &constructor2error_handler,
                    &response_transformers,
                ),
            );
        }

        let request_scoped_framework_bindings =
            framework_bindings(&package_graph, &mut krate_collection);

        let runtime_singletons: IndexSet<ResolvedType> = get_required_singleton_types(
            handler_call_graphs.iter(),
            &component2lifecycle,
            &request_scoped_framework_bindings,
        )
        // TODO: produce a proper diagnostic here
        .map_err(|e| vec![miette!(e)])?;
        let runtime_singleton_bindings = runtime_singletons
            .iter()
            .enumerate()
            // Assign a unique name to each singleton
            .map(|(i, type_)| (format_ident!("s{}", i), type_.to_owned()))
            .collect();

        // All singletons stored in the application state (i.e. all "runtime" singletons) must
        // implement `Clone`, `Send` and `Sync` in order to be shared across threads.
        let send = process_framework_path("core::marker::Send", &package_graph, &krate_collection);
        let sync = process_framework_path("core::marker::Sync", &package_graph, &krate_collection);
        let clone = process_framework_path("core::clone::Clone", &package_graph, &krate_collection);
        for singleton_type in runtime_singletons.iter().filter_map(|ty| {
            if component2lifecycle.get(ty) == Some(&Lifecycle::Singleton) {
                Some(ty)
            } else {
                None
            }
        }) {
            for trait_ in [&send, &sync, &clone] {
                if let Err(e) =
                    assert_trait_is_implemented(&krate_collection, singleton_type, trait_)
                {
                    diagnostics.push(e
                        .into_diagnostic(
                            &constructors,
                            &constructor_callable_resolver,
                            &resolved_paths2identifiers,
                            &app_blueprint.constructor_locations,
                            &package_graph,
                            Some("All singletons must implement the `Send`, `Sync` and `Clone` traits.\n \
                                `pavex` runs on a multi-threaded HTTP server and singletons must be shared \
                                 across all worker threads.".into()),
                        )
                        .to_miette());
                }
            }
        }

        exit_on_errors!(diagnostics);

        let application_state_call_graph = application_state_call_graph(
            &runtime_singleton_bindings,
            &component2lifecycle,
            constructors,
            &constructor2error_handler,
        );
        let codegen_types = codegen_types(&package_graph, &mut krate_collection);
        assert!(diagnostics.is_empty());
        Ok(App {
            package_graph,
            router,
            handler_call_graphs,
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
        );
        let generated_app_package_id = PackageId::new(GENERATED_APP_PACKAGE_ID);
        let toolchain_package_ids = TOOLCHAIN_CRATES
            .iter()
            .map(|p| PackageId::new(*p))
            .collect::<Vec<_>>();
        for package_id in &toolchain_package_ids {
            package_ids2deps.insert(&package_id, package_id.repr().into());
        }
        package_ids2deps.insert(&generated_app_package_id, "crate".into());

        let lib_rs = codegen::codegen_app(
            &self.router,
            &self.handler_call_graphs,
            &self.application_state_call_graph,
            &self.request_scoped_framework_bindings,
            &package_ids2deps,
            &self.runtime_singleton_bindings,
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
        );
        // TODO: dry this up in one place.
        let generated_app_package_id = PackageId::new(GENERATED_APP_PACKAGE_ID);
        let toolchain_package_ids = TOOLCHAIN_CRATES
            .iter()
            .map(|p| PackageId::new(*p))
            .collect::<Vec<_>>();
        for package_id in &toolchain_package_ids {
            package_ids2deps.insert(&package_id, package_id.repr().into());
        }
        package_ids2deps.insert(&generated_app_package_id, "crate".into());

        for (route, handler) in &self.router {
            let handler_call_graph = &self.handler_call_graphs[&handler.path];
            handler_graphs.insert(
                route.to_owned(),
                handler_call_graph
                    .dot(&package_ids2deps)
                    .replace("digraph", &format!("digraph \"{}\"", route)),
            );
        }
        let application_state_graph = self
            .application_state_call_graph
            .call_graph
            .dot(&package_ids2deps)
            .replace("digraph", "digraph app_state");
        AppDiagnostics {
            handlers: handler_graphs,
            application_state: application_state_graph,
        }
    }
}

/// A representation of an `App` geared towards debugging and testing.
///
/// It contains the DOT representation of all the call graphs underpinning the originating `App`.
/// The DOT representation can be used for snapshot testing and/or troubleshooting.
pub struct AppDiagnostics {
    pub handlers: IndexMap<String, String>,
    pub application_state: String,
}

impl AppDiagnostics {
    /// Persist the diagnostic information to disk, using one file per handler within the specified
    /// directory.
    pub fn persist(&self, directory: &Path) -> Result<(), anyhow::Error> {
        let handler_directory = directory.join("handlers");
        fs_err::create_dir_all(&handler_directory)?;
        for (route, handler) in &self.handlers {
            let path = handler_directory.join(format!("{}.dot", route).trim_start_matches('/'));
            let mut file = fs_err::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)?;
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
    handler_call_graphs: impl Iterator<Item = (&'a ResolvedPath, &'a CallGraph)>,
    component2lifecycle: &HashMap<ResolvedType, Lifecycle>,
    types_provided_by_the_framework: &BiHashMap<Ident, ResolvedType>,
) -> Result<IndexSet<ResolvedType>, anyhow::Error> {
    let mut singletons_to_be_built = IndexSet::new();
    for (import_path, handler_call_graph) in handler_call_graphs {
        for mut required_input in handler_call_graph.required_input_types() {
            // We don't care if the type is required as a shared reference or an owned instance here.
            // We care about the underlying type.
            required_input.is_shared_reference = false;
            if !types_provided_by_the_framework.contains_right(&required_input) {
                match component2lifecycle.get(&required_input) {
                    Some(lifecycle) => {
                        if lifecycle == &Lifecycle::Singleton {
                            singletons_to_be_built.insert(required_input);
                        }
                    }
                    None => {
                        return Err(
                            anyhow::anyhow!(
                                    "One of your handlers ({}) needs an instance of type {:?} as input \
                                    (either directly or indirectly), but there is no constructor registered for that type.",
                                    import_path, required_input
                                )
                        );
                    }
                }
            }
        }
    }
    Ok(singletons_to_be_built)
}

/// Return the set of name bindings injected by `pavex` into the processing context for
/// an incoming request (e.g. the incoming request itself!).  
/// The types injected here can be used by constructors and handlers even though no constructor
/// has been explicitly registered for them by the developer.
fn framework_bindings(
    package_graph: &PackageGraph,
    krate_collection: &mut CrateCollection,
) -> BiHashMap<Ident, ResolvedType> {
    let http_request = "pavex_runtime::http::Request::<pavex_runtime::hyper::Body>";
    let http_request = process_framework_path(http_request, package_graph, krate_collection);
    BiHashMap::from_iter([(format_ident!("request"), http_request)].into_iter())
}

/// Return the set of types that will be used in the generated code to build a functional
/// server scaffolding.  
fn codegen_types(
    package_graph: &PackageGraph,
    krate_collection: &mut CrateCollection,
) -> HashSet<ResolvedType> {
    let error = process_framework_path("pavex_runtime::Error", package_graph, krate_collection);
    HashSet::from([error])
}

/// Resolve a type path assuming that the crate is a dependency of `pavex_builder`.
fn process_framework_path(
    raw_path: &str,
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
) -> ResolvedType {
    // We are relying on a little hack to anchor our search:
    // all framework types belong to crates that are direct dependencies of `pavex_builder`.
    // TODO: find a better way in the future.
    let identifiers =
        RawCallableIdentifiers::from_raw_parts(raw_path.into(), "pavex_builder".into());
    let path = ResolvedPath::parse(&identifiers, package_graph).unwrap();
    let (item, _) = path.find_rustdoc_items(krate_collection).unwrap();
    resolve_type_path(&path, &item.item, krate_collection).unwrap()
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum BuildError {
    #[error(transparent)]
    HandlerError(#[from] Box<CallableResolutionError>),
    #[error(transparent)]
    GenericError(#[from] anyhow::Error),
}

trait ResultExt {
    fn to_miette(self) -> miette::Error;
}

impl ResultExt for Result<CompilerDiagnostic, miette::Error> {
    fn to_miette(self) -> miette::Error {
        match self {
            Ok(e) => e.into(),
            Err(e) => e,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("I cannot use the type returned by this {callable_type} to create an HTTP response.")]
pub(crate) struct OutputCannotBeConvertedIntoAResponse {
    identifiers: RawCallableIdentifiers,
    output_type: ResolvedType,
    callable_type: CallableType,
    #[source]
    source: MissingTraitImplementationError,
}

impl OutputCannotBeConvertedIntoAResponse {
    pub fn into_diagnostic(
        self,
        app_blueprint: &AppBlueprint,
        package_graph: &PackageGraph,
    ) -> Result<CompilerDiagnostic, miette::Error> {
        let location = get_registration_location(app_blueprint, &self.identifiers).unwrap();
        let source = location.source_file(&package_graph)?;
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .map(|s| s.labeled(format!("The {} was registered here", self.callable_type)));
        let diagnostic = CompilerDiagnostic::builder(source, self)
            .optional_label(label)
            .help(format!(
                "Implement `pavex_runtime::response::IntoResponse` for {:?}.",
                self.output_type
            ))
            .build();
        Ok(diagnostic)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("You registered an error handler for a constructor that does not return a `Result`.")]
pub(crate) struct ErrorHandlerForInfallibleConstructor {
    error_handler_id: RawCallableIdentifiers,
}

impl ErrorHandlerForInfallibleConstructor {
    pub fn into_diagnostic(
        self,
        app_blueprint: &AppBlueprint,
        package_graph: &PackageGraph,
    ) -> Result<CompilerDiagnostic, miette::Error> {
        let location = &app_blueprint.error_handler_locations[&self.error_handler_id];
        let source = location.source_file(&package_graph)?;
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .map(|s| s.labeled("The unnecessary error handler was registered here".into()));
        let diagnostic = CompilerDiagnostic::builder(source, self)
            .optional_label(label)
            .help(
                "Remove the error handler, it is not needed. The constructor is infallible!".into(),
            )
            .build();
        Ok(diagnostic)
    }
}
