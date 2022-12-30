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

use crate::language::ResolvedPath;
use crate::language::{Callable, ParseError, ResolvedType};
use crate::rustdoc::CrateCollection;
use crate::rustdoc::TOOLCHAIN_CRATES;
use crate::web::call_graph::CallGraph;
use crate::web::call_graph::{application_state_call_graph, handler_call_graph};
use crate::web::constructors::{Constructor, ConstructorValidationError};
use crate::web::diagnostic::{
    CompilerDiagnosticBuilder, OptionalSourceSpanExt, ParsedSourceFile, SourceSpanExt,
};
use crate::web::error_handlers::ErrorHandler;
use crate::web::generated_app::GeneratedApp;
use crate::web::resolvers::{resolve_type_path, CallableResolutionError, CallableType};
use crate::web::traits::assert_trait_is_implemented;
use crate::web::{codegen, diagnostic, resolvers, utils};

pub(crate) const GENERATED_APP_PACKAGE_ID: &str = "crate";

pub struct App {
    package_graph: PackageGraph,
    router: BTreeMap<String, Callable>,
    handler_call_graphs: IndexMap<ResolvedPath, CallGraph>,
    application_state_call_graph: CallGraph,
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

impl App {
    #[tracing::instrument(skip_all)]
    pub fn build(app_blueprint: AppBlueprint) -> Result<Self, miette::Error> {
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

        let package_graph = compute_package_graph()?;
        let mut krate_collection = CrateCollection::new(package_graph.clone());

        let resolved_paths2identifiers: HashMap<ResolvedPath, HashSet<RawCallableIdentifiers>> = {
            let mut map: HashMap<ResolvedPath, HashSet<RawCallableIdentifiers>> = HashMap::new();
            for raw_identifier in &raw_identifiers_db {
                match ResolvedPath::parse(raw_identifier, &package_graph) {
                    Ok(p) => {
                        map.entry(p).or_default().insert(raw_identifier.to_owned());
                    }
                    Err(e) => {
                        let identifiers = e.raw_identifiers();
                        let location = app_blueprint
                            .constructor_locations
                            .get(identifiers)
                            .or_else(|| {
                                app_blueprint.request_handler_locations[identifiers].first()
                            })
                            .unwrap();
                        let source = ParsedSourceFile::new(
                            location.file.as_str().into(),
                            &package_graph.workspace(),
                        )
                        .map_err(miette::MietteError::IoError)?;
                        let source_span = diagnostic::get_f_macro_invocation_span(
                            &source.contents,
                            &source.parsed,
                            location,
                        );
                        let diagnostic = match e {
                            ParseError::InvalidPath(_) => {
                                let label = source_span
                                    .labeled("The invalid import path was registered here".into());
                                CompilerDiagnosticBuilder::new(source, e)
                                    .optional_label(label)
                            }
                            ParseError::PathMustBeAbsolute(_) => {
                                let label = source_span
                                    .labeled("The relative import path was registered here".into());
                                CompilerDiagnosticBuilder::new(source, e)
                                    .optional_label(label)
                                    .help("If it is a local import, the path must start with `crate::`.\nIf it is an import from a dependency, the path must start with the dependency name (e.g. `dependency::`).".into())
                            }
                        }.build();
                        return Err(diagnostic.into());
                    }
                }
            }
            map
        };

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

        let constructor_callable_resolver =
            match resolvers::resolve_constructors(&constructor_paths, &mut krate_collection) {
                Ok(r) => r,
                Err(e) => {
                    return Err(e.into_diagnostic(
                        &resolved_paths2identifiers,
                        |identifiers| app_blueprint.constructor_locations[identifiers].clone(),
                        &package_graph,
                        CallableType::Constructor,
                    )?);
                }
            };
        let mut constructors: IndexMap<ResolvedType, Constructor> = IndexMap::new();
        for callable in constructor_callable_resolver.right_values() {
            let constructor: Constructor = match callable.to_owned().try_into() {
                Ok(c) => c,
                Err(e) => {
                    return match e {
                        ConstructorValidationError::CannotReturnTheUnitType(
                            ref constructor_path,
                        ) => {
                            let raw_identifier = resolved_paths2identifiers[constructor_path]
                                .iter()
                                .next()
                                .unwrap();
                            let location = &app_blueprint.constructor_locations[raw_identifier];
                            let source = ParsedSourceFile::new(
                                location.file.as_str().into(),
                                &package_graph.workspace(),
                            )
                            .map_err(miette::MietteError::IoError)?;
                            let label = diagnostic::get_f_macro_invocation_span(
                                &source.contents,
                                &source.parsed,
                                location,
                            )
                            .map(|s| s.labeled("The constructor was registered here".into()));
                            let diagnostic = CompilerDiagnosticBuilder::new(source, e)
                                .optional_label(label)
                                .build();
                            Err(diagnostic.into())
                        }
                    };
                }
            };
            constructors.insert(constructor.output_type().to_owned(), constructor);
        }

        let error_handler_callable_resolver =
            match resolvers::resolve_error_handlers(&error_handler_paths, &mut krate_collection) {
                Ok(r) => r,
                Err(e) => {
                    return Err(e.into_diagnostic(
                        &resolved_paths2identifiers,
                        |identifiers| app_blueprint.constructor_locations[identifiers].clone(),
                        &package_graph,
                        CallableType::ErrorHandler,
                    )?);
                }
            };

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

        let (handler_resolver, handlers) = match resolvers::resolve_request_handlers(
            &request_handler_paths,
            &mut krate_collection,
        ) {
            Ok(h) => h,
            Err(e) => {
                return Err(e.into_diagnostic(
                    &resolved_paths2identifiers,
                    |identifiers| {
                        app_blueprint.request_handler_locations[identifiers]
                            .first()
                            .unwrap()
                            .clone()
                    },
                    &package_graph,
                    CallableType::Handler,
                )?);
            }
        };

        let mut response_transformers = HashMap::<ResolvedType, Callable>::new();
        let into_response = process_framework_path(
            "pavex_runtime::response::IntoResponse",
            &package_graph,
            &krate_collection,
        );
        for handler in &handlers {
            if let Some(output) = &handler.output {
                if response_transformers.get(output).is_some() {
                    // We already processed this type
                    continue;
                }
                // Verify that the output type implements the `IntoResponse` trait.
                if let Err(e) =
                    assert_trait_is_implemented(&krate_collection, output, &into_response)
                {
                    panic!(
                        "All handler output types must implement `IntoResponse`: {:?}\n{}",
                        e, e
                    );
                }
                // todo!()
            }
        }

        // TODO: check that the error handler associated with a constructor that returns
        //  Result<_, E> has &E as one of its input types.

        let constructor2error_handler: HashMap<Constructor, ErrorHandler> = {
            let mut map = HashMap::new();
            for (output_type, constructor) in &constructors {
                if let Constructor::Callable(callable) = constructor {
                    if utils::is_result(&output_type) {
                        let constructor_path = constructor_callable_resolver
                            .get_by_right(&callable)
                            .unwrap();
                        let constructor_id =
                            constructor_paths2ids.get_by_left(constructor_path).unwrap();
                        let error_handler_id =
                            &app_blueprint.constructor_error_handlers[constructor_id];
                        let error_handler_path = &identifiers2path[error_handler_id];
                        let error_handler = error_handler_callable_resolver
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

        let mut router = BTreeMap::new();
        for (route, callable_identifiers) in app_blueprint.router {
            let callable_path = &identifiers2path[&callable_identifiers];
            router.insert(route, handler_resolver[callable_path].to_owned());
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

        let mut handler_call_graphs = IndexMap::with_capacity(handlers.len());
        for callable in &handlers {
            handler_call_graphs.insert(
                callable.path.clone(),
                handler_call_graph(
                    callable.to_owned(),
                    &component2lifecycle,
                    &constructors,
                    &constructor2error_handler,
                ),
            );
        }

        let request_scoped_framework_bindings =
            framework_bindings(&package_graph, &mut krate_collection);

        let runtime_singletons: BiHashMap<Ident, ResolvedType> = get_required_singleton_types(
            handler_call_graphs.iter(),
            &component2lifecycle,
            &request_scoped_framework_bindings,
        )
        .map_err(|e| miette!(e))?
        .into_iter()
        .enumerate()
        // Assign a unique name to each singleton
        .map(|(i, type_)| (format_ident!("s{}", i), type_))
        .collect();

        // All singletons stored in the application state (i.e. all "runtime" singletons) must
        // implement `Clone`, `Send` and `Sync` in order to be shared across threads.
        let send = process_framework_path("core::marker::Send", &package_graph, &krate_collection);
        let sync = process_framework_path("core::marker::Sync", &package_graph, &krate_collection);
        let clone = process_framework_path("core::clone::Clone", &package_graph, &krate_collection);
        for singleton_type in runtime_singletons.right_values().filter_map(|ty| {
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
                    return Err(e
                        .into_diagnostic(
                            &constructors,
                            &constructor_callable_resolver,
                            &resolved_paths2identifiers,
                            &app_blueprint.constructor_locations,
                            &package_graph,
                            Some("All singletons must implement the `Send`, `Sync` and `Clone` traits.\n \
                                `pavex` runs on a multi-threaded HTTP server and singletons must be shared \
                                 across all worker threads.".into()),
                        )?
                        .into());
                }
            }
        }

        let application_state_call_graph = application_state_call_graph(
            &runtime_singletons,
            &component2lifecycle,
            constructors,
            &constructor2error_handler,
        );
        let codegen_types = codegen_types(&package_graph, &mut krate_collection);
        Ok(App {
            package_graph,
            router,
            handler_call_graphs,
            application_state_call_graph,
            runtime_singleton_bindings: runtime_singletons,
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
            &self.application_state_call_graph,
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
            &self.application_state_call_graph,
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
) -> Result<HashSet<ResolvedType>, anyhow::Error> {
    let mut singletons_to_be_built = HashSet::new();
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
