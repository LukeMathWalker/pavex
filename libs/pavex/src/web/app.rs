use std::borrow::Cow;
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

use pavex_builder::Lifecycle;
use pavex_builder::{AppBlueprint, RawCallableIdentifiers};

use crate::language::ResolvedPath;
use crate::language::{Callable, ParseError, ResolvedType};
use crate::rustdoc::{CrateCollection, STD_PACKAGE_ID};
use crate::web::application_state_call_graph::ApplicationStateCallGraph;
use crate::web::constructors::{Constructor, ConstructorValidationError};
use crate::web::dependency_graph::CallableDependencyGraph;
use crate::web::diagnostic::{
    CompilerDiagnosticBuilder, OptionalSourceSpanExt, ParsedSourceFile, SourceSpanExt,
};
use crate::web::generated_app::GeneratedApp;
use crate::web::handler_call_graph::HandlerCallGraph;
use crate::web::resolvers::{CallableResolutionError, CallableType};
use crate::web::{codegen, diagnostic, resolvers};

pub(crate) const GENERATED_APP_PACKAGE_ID: &str = "crate";

pub struct App {
    package_graph: PackageGraph,
    router: BTreeMap<String, Callable>,
    handler_call_graphs: IndexMap<ResolvedPath, HandlerCallGraph>,
    application_state_call_graph: ApplicationStateCallGraph,
    request_scoped_framework_bindings: BiHashMap<Ident, ResolvedType>,
}

impl App {
    pub fn build(app_blueprint: AppBlueprint) -> Result<Self, miette::Error> {
        // We collect all the unique raw identifiers from the blueprint.
        let raw_identifiers_db: HashSet<RawCallableIdentifiers> = {
            let mut set = HashSet::with_capacity(
                app_blueprint.handlers.len() + app_blueprint.constructors.len(),
            );
            set.extend(app_blueprint.handlers.iter().cloned());
            set.extend(app_blueprint.constructors.iter().cloned());
            set
        };

        // `cargo metadata` seems to be the only reliable way of retrieving the path to
        // the root manifest of the current workspace for a Rust project.
        let package_graph = guppy::MetadataCommand::new()
            .exec()
            .map_err(|e| miette!(e))?
            .build_graph()
            .map_err(|e| miette!(e))?;
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
                            .or_else(|| app_blueprint.handler_locations[identifiers].first())
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

        let constructor_paths = {
            let mut set = IndexSet::with_capacity(app_blueprint.constructors.len());
            for constructor_identifiers in &app_blueprint.constructors {
                let constructor_path = identifiers2path[constructor_identifiers].clone();
                set.insert(constructor_path);
            }
            set
        };

        let constructor_paths2ids = {
            let mut set = BiHashMap::with_capacity(app_blueprint.constructors.len());
            for constructor_identifiers in &app_blueprint.constructors {
                let constructor_path = identifiers2path[constructor_identifiers].clone();
                set.insert(constructor_path, constructor_identifiers.to_owned());
            }
            set
        };

        let handler_paths = {
            let mut set = IndexSet::with_capacity(app_blueprint.handlers.len());
            for handler in &app_blueprint.handlers {
                set.insert(identifiers2path[handler].clone());
            }
            set
        };

        let (constructor_callable_resolver, constructor_callables) =
            match resolvers::resolve_constructors(
                &constructor_paths,
                &mut krate_collection,
                &package_graph,
            ) {
                Ok((resolver, constructors)) => (resolver, constructors),
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
        for (output_type, callable) in constructor_callables.into_iter() {
            let constructor = match callable.try_into() {
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
            constructors.insert(output_type, constructor);
        }

        // For each non-reference type, register an inlineable constructor that transforms
        // `T` in `&T`.
        let constructibile_types: Vec<ResolvedType> =
            constructors.keys().map(|t| t.to_owned()).collect();
        for t in constructibile_types {
            if !t.is_shared_reference {
                let c = Constructor::shared_borrow(t);
                constructors.insert(c.output_type().to_owned(), c);
            }
        }

        let (handler_resolver, handlers) = match resolvers::resolve_handlers(
            &handler_paths,
            &mut krate_collection,
            &package_graph,
        ) {
            Ok(h) => h,
            Err(e) => {
                return Err(e.into_diagnostic(
                    &resolved_paths2identifiers,
                    |identifiers| {
                        app_blueprint.handler_locations[identifiers]
                            .first()
                            .unwrap()
                            .clone()
                    },
                    &package_graph,
                    CallableType::Handler,
                )?);
            }
        };

        let mut router = BTreeMap::new();
        for (route, callable_identifiers) in app_blueprint.router {
            let callable_path = &identifiers2path[&callable_identifiers];
            router.insert(route, handler_resolver[callable_path].to_owned());
        }

        let mut handler_dependency_graphs = IndexMap::with_capacity(handlers.len());
        for callable in &handlers {
            handler_dependency_graphs.insert(
                callable.path.clone(),
                CallableDependencyGraph::new(callable.to_owned(), &constructors),
            );
        }

        let component2lifecycle = {
            let mut map = HashMap::<ResolvedType, Lifecycle>::new();
            for (output_type, constructor) in &constructors {
                match constructor {
                    Constructor::BorrowSharedReference(s) => {
                        // The lifecycle of a references matches the lifecycle of the type
                        // it refers to.
                        map.insert(output_type.to_owned(), map[&s.input].clone());
                    }
                    Constructor::Callable(c) => {
                        let callable_path = constructor_callable_resolver.get_by_right(c).unwrap();
                        let raw_identifiers =
                            constructor_paths2ids.get_by_left(callable_path).unwrap();
                        let lifecycle = app_blueprint.component_lifecycles[raw_identifiers].clone();
                        map.insert(output_type.to_owned(), lifecycle);
                    }
                }
            }
            map
        };

        let handler_call_graphs: IndexMap<_, _> = handler_dependency_graphs
            .iter()
            .map(|(path, dep_graph)| {
                (
                    path.to_owned(),
                    HandlerCallGraph::new(
                        dep_graph,
                        component2lifecycle.clone(),
                        constructors.clone(),
                    ),
                )
            })
            .collect();

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

        let application_state_call_graph =
            ApplicationStateCallGraph::new(runtime_singletons, component2lifecycle, constructors);

        Ok(App {
            package_graph,
            router,
            handler_call_graphs,
            application_state_call_graph,
            request_scoped_framework_bindings,
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
        );
        let generated_app_package_id = PackageId::new(GENERATED_APP_PACKAGE_ID);
        let std_package_id = PackageId::new(STD_PACKAGE_ID);
        package_ids2deps.insert(&generated_app_package_id, "crate".into());
        package_ids2deps.insert(&std_package_id, "std".into());

        let lib_rs = codegen::codegen_app(
            &self.router,
            &self.handler_call_graphs,
            &self.application_state_call_graph,
            &self.request_scoped_framework_bindings,
            &package_ids2deps,
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
        );
        // TODO: dry this up in one place.
        let generated_app_package_id = PackageId::new(GENERATED_APP_PACKAGE_ID);
        let std_package_id = PackageId::new(STD_PACKAGE_ID);
        package_ids2deps.insert(&generated_app_package_id, "crate".into());
        package_ids2deps.insert(&std_package_id, "std".into());

        for (route, handler) in &self.router {
            let handler_call_graph = &self.handler_call_graphs[&handler.path];
            handler_graphs.insert(
                route.to_owned(),
                handler_call_graph
                    .dot(&package_ids2deps)
                    .replace("digraph", &format!("digraph \"{}\"", route)),
            );
        }
        let application_state_graph = self.application_state_call_graph.dot(&package_ids2deps);
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
    handler_call_graphs: impl Iterator<Item = (&'a ResolvedPath, &'a HandlerCallGraph)>,
    component2lifecycle: &HashMap<ResolvedType, Lifecycle>,
    types_provided_by_the_framework: &BiHashMap<Ident, ResolvedType>,
) -> Result<HashSet<ResolvedType>, anyhow::Error> {
    let mut singletons_to_be_built = HashSet::new();
    for (import_path, handler_call_graph) in handler_call_graphs {
        for required_input in &handler_call_graph.input_parameter_types {
            // We don't care if the type is required as a shared reference or an owned instance here.
            // We care about the underlying type.
            let required_input = if required_input.is_shared_reference {
                let mut r = required_input.clone();
                r.is_shared_reference = false;
                Cow::Owned(r)
            } else {
                Cow::Borrowed(required_input)
            };
            if !types_provided_by_the_framework.contains_right(&required_input) {
                match component2lifecycle.get(&required_input) {
                    Some(lifecycle) => {
                        if lifecycle == &Lifecycle::Singleton {
                            singletons_to_be_built.insert(required_input.into_owned());
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
    fn process_framework_path(
        raw_path: &str,
        package_graph: &PackageGraph,
        krate_collection: &mut CrateCollection,
    ) -> ResolvedType {
        let identifiers =
            RawCallableIdentifiers::from_raw_parts(raw_path.into(), "pavex_runtime".into());
        let path = ResolvedPath::parse(&identifiers, package_graph).unwrap();
        let type_ = path.find_type(krate_collection).unwrap();
        let package_id = krate_collection
            .get_defining_package_id_for_item(&path.package_id, &type_.id)
            .unwrap();
        let type_base_path = krate_collection
            .get_canonical_import_path(&path.package_id, &type_.id)
            .unwrap();
        ResolvedType {
            package_id,
            base_type: type_base_path,
            generic_arguments: vec![],
            is_shared_reference: false,
        }
    }

    // http::request::Request<hyper::body::Body>
    let http_request = "http::request::Request";
    let mut http_request = process_framework_path(http_request, package_graph, krate_collection);
    let hyper_body = "hyper::body::Body";
    let hyper_body = process_framework_path(hyper_body, package_graph, krate_collection);
    http_request.generic_arguments = vec![hyper_body];

    BiHashMap::from_iter([(format_ident!("request"), http_request)].into_iter())
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum BuildError {
    #[error(transparent)]
    HandlerError(#[from] Box<CallableResolutionError>),
    #[error(transparent)]
    GenericError(#[from] anyhow::Error),
}
