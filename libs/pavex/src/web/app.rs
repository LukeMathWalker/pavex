use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
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
use quote::{format_ident, quote};
use rustdoc_types::{GenericArg, GenericArgs, ItemEnum, Type};
use syn::spanned::Spanned;
use syn::FnArg;

use pavex_builder::Lifecycle;
use pavex_builder::{AppBlueprint, RawCallableIdentifiers};

use crate::language::{Callable, ParseError, ResolvedType};
use crate::language::{EncodedResolvedPath, ResolvedPath, UnknownPath};
use crate::rustdoc::{CannotGetCrateData, Crate, CrateCollection};
use crate::web::application_state_call_graph::ApplicationStateCallGraph;
use crate::web::dependency_graph::CallableDependencyGraph;
use crate::web::diagnostic::{
    convert_rustdoc_span, convert_span, read_source_file, CompilerDiagnosticBuilder,
    ParsedSourceFile, SourceSpanExt,
};
use crate::web::handler_call_graph::HandlerCallGraph;
use crate::web::{codegen, diagnostic};

pub(crate) const STD_PACKAGE_ID: &str = "std";
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
                    Err(ParseError::InvalidPath(e)) => {
                        let location = app_blueprint
                            .constructor_locations
                            .get(raw_identifier)
                            .or_else(|| app_blueprint.handler_locations[raw_identifier].first())
                            .unwrap();
                        let source = ParsedSourceFile::new(
                            location.file.as_str().into(),
                            &package_graph.workspace(),
                        )
                        .map_err(miette::MietteError::IoError)?;
                        let label = diagnostic::get_callable_invocation_span(
                            &source.contents,
                            &source.parsed,
                            location,
                        )
                        .map(|s| s.labeled("The invalid import path was registered here".into()));
                        let diagnostic = CompilerDiagnosticBuilder::new(source, e)
                            .optional_label(label)
                            .build();
                        return Err(diagnostic.into());
                    }
                    Err(ParseError::PathMustBeAbsolute(e)) => {
                        let location = app_blueprint
                            .constructor_locations
                            .get(raw_identifier)
                            .or_else(|| app_blueprint.handler_locations[raw_identifier].first())
                            .unwrap();
                        let source = ParsedSourceFile::new(
                            location.file.as_str().into(),
                            &package_graph.workspace(),
                        )
                        .map_err(miette::MietteError::IoError)?;
                        let label = diagnostic::get_callable_invocation_span(
                            &source.contents,
                            &source.parsed,
                            location,
                        )
                        .map(|s| s.labeled("The invalid import path was registered here".into()));
                        let diagnostic = CompilerDiagnosticBuilder::new(source, e)
                            .optional_label(label)
                            .help("If it is a local import, the path must start with `crate::`.\nIf it is an import from a dependency, the path must start with the dependency name (e.g. `dependency::`).".into())
                            .build();
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
            process_constructors(&constructor_paths, &mut krate_collection, &package_graph)
                .map_err(|e| miette!(e))?;

        let (handler_resolver, handlers) = match process_handlers(
            &handler_paths,
            &mut krate_collection,
            &package_graph,
        ) {
            Ok(h) => h,
            Err(e) => {
                return match e {
                    CallableResolutionError::UnknownCallable(e) => {
                        // We only report a single registration site in the error report even though
                        // the same callable might have been registered in multiple locations.
                        // We may or may not want to change this in the future.
                        let type_path = e.0.decode();
                        let raw_identifier = resolved_paths2identifiers[&type_path]
                            .iter()
                            .next()
                            .unwrap();
                        let location = app_blueprint.handler_locations[raw_identifier]
                            .first()
                            .unwrap();
                        let source = ParsedSourceFile::new(
                            location.file.as_str().into(),
                            &package_graph.workspace(),
                        )
                        .map_err(miette::MietteError::IoError)?;
                        let label = diagnostic::get_callable_invocation_span(
                            &source.contents,
                            &source.parsed,
                            location,
                        )
                        .map(|s| s.labeled("The callable we cannot resolve".into()));
                        let diagnostic = CompilerDiagnosticBuilder::new(source, e)
                            .optional_label(label)
                            .help("This is most likely a bug in `pavex` or `rustdoc`.\nPlease file a GitHub issue!".into())
                            .build();
                        Err(diagnostic.into())
                    }
                    CallableResolutionError::ParameterResolutionError(e) => {
                        let sub_diagnostic = {
                            if let Some(definition_span) = &e.callable_item.span {
                                let source_contents = read_source_file(
                                    &definition_span.filename,
                                    &package_graph.workspace(),
                                )
                                .map_err(miette::MietteError::IoError)?;
                                let span = convert_rustdoc_span(
                                    &source_contents,
                                    definition_span.to_owned(),
                                );
                                let span_contents =
                                    &source_contents[span.offset()..(span.offset() + span.len())];
                                let input = match &e.callable_item.inner {
                                    ItemEnum::Function(_) => {
                                        let item: syn::ItemFn =
                                            syn::parse_str(span_contents).unwrap();
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
                                    CompilerDiagnosticBuilder::new(
                                        source_code,
                                        anyhow::anyhow!(""),
                                    )
                                    .label(label)
                                    .build(),
                                )
                            } else {
                                None
                            }
                        };

                        let callable_path = e.callable_path.decode();
                        let raw_identifier = resolved_paths2identifiers[&callable_path]
                            .iter()
                            .next()
                            .unwrap();
                        let location = app_blueprint.handler_locations[raw_identifier]
                            .first()
                            .unwrap();
                        let source = ParsedSourceFile::new(
                            location.file.as_str().into(),
                            &package_graph.workspace(),
                        )
                        .map_err(miette::MietteError::IoError)?;
                        let label = diagnostic::get_callable_invocation_span(
                            &source.contents,
                            &source.parsed,
                            location,
                        )
                        .map(|s| s.labeled("The callable was registered here".into()));
                        let diagnostic = CompilerDiagnosticBuilder::new(source, e)
                            .optional_label(label)
                            .optional_related_error(sub_diagnostic)
                            .build();
                        Err(diagnostic.into())
                    }
                    CallableResolutionError::UnsupportedCallableKind(_)
                    | CallableResolutionError::CannotGetCrateData(_)
                    | CallableResolutionError::OutputTypeResolutionError(_) => Err(miette!(e)),
                };
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
        let krate = krate_collection
            .get_or_compute_by_id(&path.package_id)
            .unwrap();
        let summary = krate.get_summary_by_id(&type_.id).unwrap();
        let type_base_path = summary.path.clone();
        ResolvedType {
            package_id: path.package_id,
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
fn process_constructors(
    constructor_paths: &IndexSet<ResolvedPath>,
    krate_collection: &mut CrateCollection,
    package_graph: &guppy::graph::PackageGraph,
) -> Result<
    (
        BiHashMap<ResolvedPath, Callable>,
        IndexMap<ResolvedType, Callable>,
    ),
    anyhow::Error,
> {
    let mut resolution_map = BiHashMap::with_capacity(constructor_paths.len());
    let mut constructors = IndexMap::with_capacity(constructor_paths.len());
    for constructor_identifiers in constructor_paths {
        let constructor =
            process_constructor(constructor_identifiers, krate_collection, package_graph)?;
        // TODO: raise an error if multiple constructors are registered for the same type
        constructors.insert(constructor.output_fq_path.clone(), constructor.clone());
        resolution_map.insert(constructor_identifiers.to_owned(), constructor);
    }
    Ok((resolution_map, constructors))
}

/// Extract the input type paths, the output type path and the callable path for a type constructor.
fn process_constructor(
    constructor_path: &ResolvedPath,
    krate_collection: &mut CrateCollection,
    package_graph: &guppy::graph::PackageGraph,
) -> Result<Callable, anyhow::Error> {
    let constructor = process_callable(krate_collection, constructor_path, package_graph)?;
    if constructor.output_fq_path.base_type == vec!["()"] {
        return Err(anyhow::anyhow!(
            "A constructor cannot return the unit type!"
        ));
    }
    Ok(constructor)
}

/// Extract the input type paths, the output type path and the callable path for each
/// registered handler.
fn process_handlers(
    handler_paths: &IndexSet<ResolvedPath>,
    krate_collection: &mut CrateCollection,
    package_graph: &guppy::graph::PackageGraph,
) -> Result<(HashMap<ResolvedPath, Callable>, IndexSet<Callable>), CallableResolutionError> {
    let mut handlers = IndexSet::with_capacity(handler_paths.len());
    let mut handler_resolver = HashMap::new();
    for callable_path in handler_paths {
        let handler = process_callable(krate_collection, callable_path, package_graph)?;
        handlers.insert(handler.clone());
        handler_resolver.insert(callable_path.to_owned(), handler);
    }
    Ok((handler_resolver, handlers))
}

fn process_type(
    type_: &rustdoc_types::Type,
    krate: &Crate,
    package_graph: &guppy::graph::PackageGraph,
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
                                        krate,
                                        package_graph,
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
            let type_summary = krate.get_summary_by_id(id)?;
            let type_package_id = if type_summary.crate_id == 0 {
                krate.package_id().to_owned()
            } else {
                let (owning_crate, owning_crate_version) = krate
                    .get_external_crate_name(type_summary.crate_id)
                    .unwrap();
                if owning_crate.name == "std" {
                    PackageId::new(STD_PACKAGE_ID)
                } else {
                    let transitive_dependencies = package_graph
                        .query_forward([krate.package_id()])
                        .unwrap()
                        .resolve();
                    let mut iterator =
                        transitive_dependencies.links(guppy::graph::DependencyDirection::Forward);
                    iterator
                        .find(|link| {
                            link.to().name() == owning_crate.name
                                && owning_crate_version
                                    .as_ref()
                                    .map(|v| link.to().version() == v)
                                    .unwrap_or(true)
                        })
                        .ok_or_else(|| {
                            anyhow!(
                            "I could not find the package id for the crate where `{}` is defined",
                            type_summary.path.join("::")
                        )
                        })
                        .unwrap()
                        .to()
                        .id()
                        .to_owned()
                }
            };
            Ok(ResolvedType {
                package_id: type_package_id,
                base_type: type_summary.path.clone(),
                generic_arguments: generics,
            })
        }
        _ => Err(anyhow!(
            "We cannot handle inputs of this kind ({:?}) yet. Sorry!",
            type_
        )),
    }
}

fn process_callable(
    krate_collection: &mut CrateCollection,
    callable_path: &ResolvedPath,
    package_graph: &guppy::graph::PackageGraph,
) -> Result<Callable, CallableResolutionError> {
    let type_ = callable_path.find_type(krate_collection)?;
    let krate = krate_collection.get_or_compute_by_id(&callable_path.package_id)?;
    let decl = match &type_.inner {
        ItemEnum::Function(f) => &f.decl,
        ItemEnum::Method(m) => &m.decl,
        kind => {
            let path = &callable_path.path.0;
            return Err(UnsupportedCallableKind {
                import_path: quote! { #path }.to_string(),
                // TODO: review how this gets formatted
                item_kind: format!("{:?}", kind),
            }
            .into());
        }
    };

    let mut parameter_paths = Vec::with_capacity(decl.inputs.len());
    for (parameter_index, (_, parameter_type)) in decl.inputs.iter().enumerate() {
        match process_type(parameter_type, krate, package_graph) {
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
            // TODO: distinguish between output and parameters
            match process_type(output_type, krate, package_graph) {
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

#[derive(Debug, thiserror::Error)]
#[error("I know how to work with non-generic free functions or static methods, but `{import_path:?}` is neither ({item_kind}).")]
pub(crate) struct UnsupportedCallableKind {
    pub import_path: String,
    pub item_kind: String,
}

#[derive(Debug, thiserror::Error)]
#[error("One of the input parameters for `{callable_path}` has a type that I cannot handle.")]
pub(crate) struct ParameterResolutionError {
    pub callable_path: EncodedResolvedPath,
    pub callable_item: rustdoc_types::Item,
    pub parameter_type: rustdoc_types::Type,
    pub parameter_index: usize,
    #[source]
    pub source: anyhow::Error,
}

#[derive(Debug, thiserror::Error)]
#[error("I do not know how to handle the type returned by `{callable_path}`.")]
pub(crate) struct OutputTypeResolutionError {
    pub callable_path: EncodedResolvedPath,
    pub callable_item: rustdoc_types::Item,
    pub output_type: rustdoc_types::Type,
    #[source]
    pub source: anyhow::Error,
}
