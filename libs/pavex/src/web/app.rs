use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Debug;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use bimap::BiHashMap;
use guppy::graph::PackageGraph;
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};
use miette::{miette, NamedSource, SourceSpan};
use proc_macro2::{Ident, TokenStream};
use quote::format_ident;
use rustdoc_types::{GenericArg, GenericArgs, ItemEnum, Type};
use syn::spanned::Spanned;
use syn::{FnArg, ReturnType};

use pavex_builder::{AppBlueprint, RawCallableIdentifiers};
use pavex_builder::{Lifecycle, Location};

use crate::language::{Callable, ParseError, ResolvedType};
use crate::language::{ResolvedPath, UnknownPath};
use crate::rustdoc::{CannotGetCrateData, CrateCollection, STD_PACKAGE_ID};
use crate::web::application_state_call_graph::ApplicationStateCallGraph;
use crate::web::dependency_graph::CallableDependencyGraph;
use crate::web::diagnostic::{
    convert_rustdoc_span, convert_span, read_source_file, CompilerDiagnosticBuilder,
    OptionalSourceSpanExt, ParsedSourceFile, SourceSpanExt,
};
use crate::web::handler_call_graph::HandlerCallGraph;
use crate::web::{codegen, diagnostic};

pub(crate) const GENERATED_APP_PACKAGE_ID: &str = "crate";

pub struct App {
    package_graph: PackageGraph,
    router: BTreeMap<String, Callable>,
    handler_call_graphs: IndexMap<ResolvedPath, HandlerCallGraph>,
    application_state_call_graph: ApplicationStateCallGraph,
    request_scoped_framework_bindings: BiHashMap<Ident, ResolvedType>,
}

#[derive(Clone)]
pub struct GeneratedApp {
    pub lib_rs: TokenStream,
    pub cargo_toml: cargo_manifest::Manifest,
}

impl GeneratedApp {
    pub fn persist(mut self, directory: &Path) -> Result<(), anyhow::Error> {
        // `cargo metadata` seems to be the only reliable way of retrieving the path to
        // the root manifest of the current workspace for a Rust project.
        let package_graph = guppy::MetadataCommand::new().exec()?.build_graph()?;
        let workspace = package_graph.workspace();

        let directory = if directory.is_relative() {
            workspace.root().as_std_path().join(directory)
        } else {
            directory.to_path_buf()
        };

        Self::inject_app_into_workspace_members(&workspace, &directory)?;

        let lib_rs = prettyplease::unparse(&syn::parse2(self.lib_rs)?);

        if let Some(dependencies) = &mut self.cargo_toml.dependencies {
            for dependency in dependencies.values_mut() {
                if let cargo_manifest::Dependency::Detailed(detailed) = dependency {
                    if let Some(path) = &mut detailed.path {
                        let parsed_path = PathBuf::from(path.to_owned());
                        let relative_path = pathdiff::diff_paths(parsed_path, &directory).unwrap();
                        *path = relative_path.to_string_lossy().to_string();
                    }
                }
            }
        }
        let cargo_toml = toml::to_string(&self.cargo_toml)?;
        let cargo_toml_path = directory.join("Cargo.toml");
        let source_directory = directory.join("src");
        fs_err::create_dir_all(&source_directory)?;
        fs_err::write(source_directory.join("lib.rs"), lib_rs)?;
        fs_err::write(cargo_toml_path, cargo_toml)?;
        Ok(())
    }

    // Inject the newly generated crate in the list of members for the current workspace
    fn inject_app_into_workspace_members(
        workspace: &guppy::graph::Workspace,
        generated_crate_directory: &Path,
    ) -> Result<(), anyhow::Error> {
        let root_path = workspace.root().as_std_path();
        let root_manifest_path = root_path.join("Cargo.toml");
        let root_manifest = fs_err::read_to_string(&root_manifest_path)?;
        let mut root_manifest: toml::Value = toml::from_str(&root_manifest)?;

        let member_path = pathdiff::diff_paths(&generated_crate_directory, root_path)
            .unwrap()
            .to_string_lossy()
            .to_string();

        if root_manifest.get("workspace").is_none() {
            let root_manifest = root_manifest.as_table_mut().unwrap();
            let members = toml::Value::Array(vec![".".to_string().into(), member_path.into()]);
            let mut workspace = toml::value::Table::new();
            workspace.insert("members".into(), members);
            root_manifest.insert("workspace".into(), workspace.into());
        } else {
            let workspace = root_manifest
                .get_mut("workspace")
                .unwrap()
                .as_table_mut()
                .unwrap();
            if let Some(members) = workspace.get_mut("members") {
                if let Some(members) = members.as_array_mut() {
                    if members
                        .iter()
                        .find(|m| m.as_str() == Some(&member_path))
                        .is_none()
                    {
                        members.push(member_path.into());
                    }
                }
            } else {
                let members = toml::Value::Array(vec![".".to_string().into(), member_path.into()]);
                workspace.insert("members".into(), members);
            }
        }
        let mut root_manifest_file = fs_err::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&root_manifest_path)?;
        root_manifest_file.write_all(toml::to_string(&root_manifest)?.as_bytes())?;
        Ok(())
    }
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

        let (constructor_resolver, constructors) =
            match resolve_constructors(&constructor_paths, &mut krate_collection, &package_graph) {
                Ok((resolver, constructors)) => (resolver, constructors),
                Err(e) => {
                    return match e {
                        ConstructorResolutionError::CallableResolutionError(e) => Err(e
                            .into_diagnostic(
                                &resolved_paths2identifiers,
                                |identifiers| {
                                    app_blueprint.constructor_locations[identifiers].clone()
                                },
                                &package_graph,
                            )?
                            .into()),
                    };
                }
            };

        if let Err(e) = validate_constructors(&constructors) {
            return match e {
                ConstructorValidationError::CannotReturnTheUnitType(ref constructor_path) => {
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

        let (handler_resolver, handlers) =
            match resolve_handlers(&handler_paths, &mut krate_collection, &package_graph) {
                Ok(h) => h,
                Err(e) => {
                    return Err(e
                        .into_diagnostic(
                            &resolved_paths2identifiers,
                            |identifiers| {
                                app_blueprint.handler_locations[identifiers]
                                    .first()
                                    .unwrap()
                                    .clone()
                            },
                            &package_graph,
                        )?
                        .into());
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
                callable.callable_fq_path.clone(),
                CallableDependencyGraph::new(callable.to_owned(), &constructors),
            );
        }

        let component2lifecycle = {
            let mut map = HashMap::<ResolvedType, Lifecycle>::new();
            for (output_type, constructor) in &constructors {
                let callable_path = constructor_resolver.get_by_right(constructor).unwrap();
                let raw_identifiers = constructor_paths2ids.get_by_left(callable_path).unwrap();
                let lifecycle = app_blueprint.component_lifecycles[raw_identifiers].clone();
                map.insert(output_type.to_owned(), lifecycle);
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
            let handler_call_graph = &self.handler_call_graphs[&handler.callable_fq_path];
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

fn get_required_singleton_types<'a>(
    handler_call_graphs: impl Iterator<Item = (&'a ResolvedPath, &'a HandlerCallGraph)>,
    component2lifecycle: &HashMap<ResolvedType, Lifecycle>,
    types_provided_by_the_framework: &BiHashMap<Ident, ResolvedType>,
) -> Result<HashSet<ResolvedType>, anyhow::Error> {
    let mut singletons_to_be_built = HashSet::new();
    for (import_path, handler_call_graph) in handler_call_graphs {
        for required_input in &handler_call_graph.input_parameter_types {
            if !types_provided_by_the_framework.contains_right(required_input) {
                match component2lifecycle.get(required_input) {
                    Some(lifecycle) => {
                        if lifecycle == &Lifecycle::Singleton {
                            singletons_to_be_built.insert(required_input.to_owned());
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

/// Extract the input type paths, the output type path and the callable path for each
/// registered type constructor.
fn resolve_constructors(
    constructor_paths: &IndexSet<ResolvedPath>,
    krate_collection: &mut CrateCollection,
    package_graph: &PackageGraph,
) -> Result<
    (
        BiHashMap<ResolvedPath, Callable>,
        IndexMap<ResolvedType, Callable>,
    ),
    ConstructorResolutionError,
> {
    let mut resolution_map = BiHashMap::with_capacity(constructor_paths.len());
    let mut constructors = IndexMap::with_capacity(constructor_paths.len());
    for constructor_identifiers in constructor_paths {
        let constructor =
            resolve_callable(krate_collection, constructor_identifiers, package_graph)?;
        constructors.insert(constructor.output_fq_path.clone(), constructor.clone());
        resolution_map.insert(constructor_identifiers.to_owned(), constructor);
    }
    Ok((resolution_map, constructors))
}

/// Validate the signature of all registered constructors.
fn validate_constructors(
    constructors: &IndexMap<ResolvedType, Callable>,
) -> Result<(), ConstructorValidationError> {
    for (_output_type, constructor) in constructors.iter() {
        validate_constructor(&constructor)?;
    }
    Ok(())
}

/// Validate the signature of a constructor
fn validate_constructor(constructor: &Callable) -> Result<(), ConstructorValidationError> {
    if constructor.output_fq_path.base_type == vec!["()"] {
        return Err(ConstructorValidationError::CannotReturnTheUnitType(
            constructor.callable_fq_path.to_owned(),
        ));
    }
    Ok(())
}

/// Extract the input type paths, the output type path and the callable path for each
/// registered handler.
fn resolve_handlers(
    handler_paths: &IndexSet<ResolvedPath>,
    krate_collection: &mut CrateCollection,
    package_graph: &PackageGraph,
) -> Result<(HashMap<ResolvedPath, Callable>, IndexSet<Callable>), CallableResolutionError> {
    let mut handlers = IndexSet::with_capacity(handler_paths.len());
    let mut handler_resolver = HashMap::new();
    for callable_path in handler_paths {
        let handler = resolve_callable(krate_collection, callable_path, package_graph)?;
        handlers.insert(handler.clone());
        handler_resolver.insert(callable_path.to_owned(), handler);
    }
    Ok((handler_resolver, handlers))
}

fn process_type(
    type_: &Type,
    // The package id where the type we are trying to process has been referenced (e.g. as an
    // input/output parameter).
    used_by_package_id: &PackageId,
    package_graph: &PackageGraph,
    krate_collection: &mut CrateCollection,
) -> Result<ResolvedType, anyhow::Error> {
    match type_ {
        Type::ResolvedPath(rustdoc_types::Path { id, args, .. }) => {
            let mut generics = vec![];
            if let Some(args) = args {
                match &**args {
                    GenericArgs::AngleBracketed { args, .. } => {
                        for arg in args {
                            match arg {
                                GenericArg::Lifetime(_) => {
                                    return Err(anyhow!(
                                        "We do not support lifetime arguments in types yet. Sorry!"
                                    ));
                                }
                                GenericArg::Type(generic_type) => {
                                    generics.push(process_type(
                                        generic_type,
                                        used_by_package_id,
                                        package_graph,
                                        krate_collection,
                                    )?);
                                }
                                GenericArg::Const(_) => {
                                    return Err(anyhow!(
                                        "We do not support const generics in types yet. Sorry!"
                                    ));
                                }
                                GenericArg::Infer => {
                                    return Err(anyhow!("We do not support inferred generic arguments in types yet. Sorry!"));
                                }
                            }
                        }
                    }
                    GenericArgs::Parenthesized { .. } => {
                        return Err(anyhow!("We do not support function pointers yet. Sorry!"));
                    }
                }
            }
            let type_package_id =
                krate_collection.get_defining_package_id_for_item(used_by_package_id, id)?;
            let base_type = krate_collection.get_canonical_import_path(used_by_package_id, id)?;
            Ok(ResolvedType {
                package_id: type_package_id,
                base_type,
                generic_arguments: generics,
            })
        }
        _ => Err(anyhow!(
            "We cannot handle inputs of this kind ({:?}) yet. Sorry!",
            type_
        )),
    }
}

fn resolve_callable(
    krate_collection: &mut CrateCollection,
    callable_path: &ResolvedPath,
    package_graph: &PackageGraph,
) -> Result<Callable, CallableResolutionError> {
    let type_ = callable_path.find_type(krate_collection)?;
    let used_by_package_id = &callable_path.package_id;
    let decl = match &type_.inner {
        ItemEnum::Function(f) => &f.decl,
        ItemEnum::Method(m) => &m.decl,
        kind => {
            let item_kind = match kind {
                ItemEnum::Module(_) => "a module",
                ItemEnum::ExternCrate { .. } => "an external crate",
                ItemEnum::Import(_) => "an import",
                ItemEnum::Union(_) => "a union",
                ItemEnum::Struct(_) => "a struct",
                ItemEnum::StructField(_) => "a struct field",
                ItemEnum::Enum(_) => "an enum",
                ItemEnum::Variant(_) => "an enum variant",
                ItemEnum::Function(_) => "a function",
                ItemEnum::Trait(_) => "a trait",
                ItemEnum::TraitAlias(_) => "a trait alias",
                ItemEnum::Method(_) => "a method",
                ItemEnum::Impl(_) => "an impl block",
                ItemEnum::Typedef(_) => "a type definition",
                ItemEnum::OpaqueTy(_) => "an opaque type",
                ItemEnum::Constant(_) => "a constant",
                ItemEnum::Static(_) => "a static",
                ItemEnum::ForeignType => "a foreign type",
                ItemEnum::Macro(_) => "a macro",
                ItemEnum::ProcMacro(_) => "a procedural macro",
                ItemEnum::PrimitiveType(_) => "a primitive type",
                ItemEnum::AssocConst { .. } => "an associated constant",
                ItemEnum::AssocType { .. } => "an associated type",
            }
            .to_string();
            return Err(UnsupportedCallableKind {
                import_path: callable_path.to_owned(),
                item_kind,
            }
            .into());
        }
    };

    let mut parameter_paths = Vec::with_capacity(decl.inputs.len());
    for (parameter_index, (_, parameter_type)) in decl.inputs.iter().enumerate() {
        match process_type(
            parameter_type,
            used_by_package_id,
            package_graph,
            krate_collection,
        ) {
            Ok(p) => parameter_paths.push(p),
            Err(e) => {
                return Err(ParameterResolutionError {
                    parameter_type: parameter_type.to_owned(),
                    callable_path: callable_path.to_owned().into(),
                    callable_item: type_,
                    source: e,
                    parameter_index,
                }
                .into());
            }
        }
    }
    let output_type_path = match &decl.output {
        // Unit type
        None => ResolvedType {
            package_id: PackageId::new(STD_PACKAGE_ID),
            base_type: vec!["()".into()],
            generic_arguments: vec![],
        },
        Some(output_type) => {
            match process_type(
                output_type,
                used_by_package_id,
                package_graph,
                krate_collection,
            ) {
                Ok(p) => p,
                Err(e) => {
                    return Err(OutputTypeResolutionError {
                        output_type: output_type.to_owned(),
                        callable_path: callable_path.to_owned().into(),
                        callable_item: type_,
                        source: e,
                    }
                    .into());
                }
            }
        }
    };
    Ok(Callable {
        output_fq_path: output_type_path,
        callable_fq_path: callable_path.to_owned(),
        inputs: parameter_paths,
    })
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum ConstructorResolutionError {
    #[error(transparent)]
    CallableResolutionError(#[from] CallableResolutionError),
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum ConstructorValidationError {
    #[error("I expect all constructors to return *something*.\nThis constructor doesn't, it returns the unit type - `()`.")]
    CannotReturnTheUnitType(ResolvedPath),
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum CallableResolutionError {
    #[error(transparent)]
    UnsupportedCallableKind(#[from] UnsupportedCallableKind),
    #[error(transparent)]
    UnknownCallable(#[from] UnknownPath),
    #[error(transparent)]
    ParameterResolutionError(#[from] ParameterResolutionError),
    #[error(transparent)]
    OutputTypeResolutionError(#[from] OutputTypeResolutionError),
    #[error(transparent)]
    CannotGetCrateData(#[from] CannotGetCrateData),
}

impl CallableResolutionError {
    pub fn into_diagnostic<LocationProvider>(
        self,
        resolved_paths2identifiers: &HashMap<ResolvedPath, HashSet<RawCallableIdentifiers>>,
        identifiers2location: LocationProvider,
        package_graph: &PackageGraph,
    ) -> Result<miette::Error, miette::Error>
    where
        LocationProvider: Fn(&RawCallableIdentifiers) -> Location,
    {
        let diagnostic = match self {
            Self::UnknownCallable(e) => {
                // We only report a single registration site in the error report even though
                // the same callable might have been registered in multiple locations.
                // We may or may not want to change this in the future.
                let type_path = &e.0;
                let raw_identifier = resolved_paths2identifiers[type_path].iter().next().unwrap();
                let location = identifiers2location(raw_identifier);
                let source = ParsedSourceFile::new(
                    location.file.as_str().into(),
                    &package_graph.workspace(),
                )
                .map_err(miette::MietteError::IoError)?;
                let label = diagnostic::get_f_macro_invocation_span(
                    &source.contents,
                    &source.parsed,
                    &location,
                )
                .map(|s| s.labeled("The handler that we cannot resolve".into()));
                let diagnostic = CompilerDiagnosticBuilder::new(source, e)
                    .optional_label(label)
                    .help("This is most likely a bug in `pavex` or `rustdoc`.\nPlease file a GitHub issue!".into())
                    .build();
                diagnostic.into()
            }
            CallableResolutionError::ParameterResolutionError(e) => {
                let sub_diagnostic = {
                    if let Some(definition_span) = &e.callable_item.span {
                        let source_contents =
                            read_source_file(&definition_span.filename, &package_graph.workspace())
                                .map_err(miette::MietteError::IoError)?;
                        let span =
                            convert_rustdoc_span(&source_contents, definition_span.to_owned());
                        let span_contents =
                            &source_contents[span.offset()..(span.offset() + span.len())];
                        let input = match &e.callable_item.inner {
                            ItemEnum::Function(_) => {
                                let item: syn::ItemFn = syn::parse_str(span_contents).unwrap();
                                let mut inputs = item.sig.inputs.iter();
                                inputs.nth(e.parameter_index).cloned()
                            }
                            ItemEnum::Method(_) => {
                                let item: syn::ImplItemMethod =
                                    syn::parse_str(span_contents).unwrap();
                                let mut inputs = item.sig.inputs.iter();
                                inputs.nth(e.parameter_index).cloned()
                            }
                            _ => unreachable!(),
                        }
                        .unwrap();
                        let s = convert_span(
                            span_contents,
                            match input {
                                FnArg::Typed(typed) => typed.ty.span(),
                                FnArg::Receiver(r) => r.span(),
                            },
                        );
                        let label = SourceSpan::new(
                            // We must shift the offset forward because it's the
                            // offset from the beginning of the file slice that
                            // we deserialized, instead of the entire file
                            (s.offset() + span.offset()).into(),
                            s.len().into(),
                        )
                        .labeled("The parameter type that I cannot handle".into());
                        let source_code = NamedSource::new(
                            &definition_span.filename.to_str().unwrap(),
                            source_contents,
                        );
                        Some(
                            CompilerDiagnosticBuilder::new(source_code, anyhow::anyhow!(""))
                                .label(label)
                                .build(),
                        )
                    } else {
                        None
                    }
                };

                let callable_path = &e.callable_path;
                let raw_identifier = resolved_paths2identifiers[callable_path]
                    .iter()
                    .next()
                    .unwrap();
                let location = identifiers2location(raw_identifier);
                let source = ParsedSourceFile::new(
                    location.file.as_str().into(),
                    &package_graph.workspace(),
                )
                .map_err(miette::MietteError::IoError)?;
                let label = diagnostic::get_f_macro_invocation_span(
                    &source.contents,
                    &source.parsed,
                    &location,
                )
                .map(|s| s.labeled("The handler was registered here".into()));
                let diagnostic = CompilerDiagnosticBuilder::new(source, e)
                    .optional_label(label)
                    .optional_related_error(sub_diagnostic)
                    .build();
                diagnostic.into()
            }
            CallableResolutionError::UnsupportedCallableKind(e) => {
                let type_path = &e.import_path;
                let raw_identifier = resolved_paths2identifiers[type_path].iter().next().unwrap();
                let location = identifiers2location(raw_identifier);
                let source = ParsedSourceFile::new(
                    location.file.as_str().into(),
                    &package_graph.workspace(),
                )
                .map_err(miette::MietteError::IoError)?;
                let label = diagnostic::get_f_macro_invocation_span(
                    &source.contents,
                    &source.parsed,
                    &location,
                )
                .map(|s| s.labeled("It was registered as a handler here".into()));
                let diagnostic = CompilerDiagnosticBuilder::new(source, e)
                    .optional_label(label)
                    .build();
                diagnostic.into()
            }
            CallableResolutionError::OutputTypeResolutionError(e) => {
                let sub_diagnostic = {
                    if let Some(definition_span) = &e.callable_item.span {
                        let source_contents =
                            read_source_file(&definition_span.filename, &package_graph.workspace())
                                .map_err(miette::MietteError::IoError)?;
                        let span =
                            convert_rustdoc_span(&source_contents, definition_span.to_owned());
                        let span_contents =
                            &source_contents[span.offset()..(span.offset() + span.len())];
                        let output = match &e.callable_item.inner {
                            ItemEnum::Function(_) => {
                                let item: syn::ItemFn = syn::parse_str(span_contents).unwrap();
                                item.sig.output
                            }
                            ItemEnum::Method(_) => {
                                let item: syn::ImplItemMethod =
                                    syn::parse_str(span_contents).unwrap();
                                item.sig.output
                            }
                            _ => unreachable!(),
                        };
                        let source_span = match output {
                            ReturnType::Default => None,
                            ReturnType::Type(_, type_) => Some(type_.span()),
                        }
                        .map(|s| {
                            let s = convert_span(span_contents, s);
                            SourceSpan::new(
                                // We must shift the offset forward because it's the
                                // offset from the beginning of the file slice that
                                // we deserialized, instead of the entire file
                                (s.offset() + span.offset()).into(),
                                s.len().into(),
                            )
                        });
                        let label =
                            source_span.labeled("The output type that I cannot handle".into());
                        let source_code = NamedSource::new(
                            &definition_span.filename.to_str().unwrap(),
                            source_contents,
                        );
                        Some(
                            CompilerDiagnosticBuilder::new(source_code, anyhow::anyhow!(""))
                                .optional_label(label)
                                .build(),
                        )
                    } else {
                        None
                    }
                };

                let callable_path = &e.callable_path;
                let raw_identifier = resolved_paths2identifiers[callable_path]
                    .iter()
                    .next()
                    .unwrap();
                let location = identifiers2location(raw_identifier);
                let source = ParsedSourceFile::new(
                    location.file.as_str().into(),
                    &package_graph.workspace(),
                )
                .map_err(miette::MietteError::IoError)?;
                let label = diagnostic::get_f_macro_invocation_span(
                    &source.contents,
                    &source.parsed,
                    &location,
                )
                .map(|s| s.labeled("The handler was registered here".into()));
                let diagnostic = CompilerDiagnosticBuilder::new(source, e)
                    .optional_label(label)
                    .optional_related_error(sub_diagnostic)
                    .build();
                diagnostic.into()
            }
            CallableResolutionError::CannotGetCrateData(e) => miette!(e),
        };
        Ok(diagnostic)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("I can work with functions and static methods, but `{import_path}` is neither.\nIt is {item_kind} and I do not know how to handle it here.")]
pub(crate) struct UnsupportedCallableKind {
    pub import_path: ResolvedPath,
    pub item_kind: String,
}

#[derive(Debug, thiserror::Error)]
#[error("One of the input parameters for `{callable_path}` has a type that I cannot handle.")]
pub(crate) struct ParameterResolutionError {
    pub callable_path: ResolvedPath,
    pub callable_item: rustdoc_types::Item,
    pub parameter_type: Type,
    pub parameter_index: usize,
    #[source]
    pub source: anyhow::Error,
}

#[derive(Debug, thiserror::Error)]
#[error("I do not know how to handle the type returned by `{callable_path}`.")]
pub(crate) struct OutputTypeResolutionError {
    pub callable_path: ResolvedPath,
    pub callable_item: rustdoc_types::Item,
    pub output_type: Type,
    #[source]
    pub source: anyhow::Error,
}
