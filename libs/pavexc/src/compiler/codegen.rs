use std::collections::BTreeMap;

use ahash::HashMap;
use bimap::{BiBTreeMap, BiHashMap};
use cargo_manifest::{Dependency, DependencyDetail, Edition};
use guppy::graph::{ExternalSource, PackageSource};
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{ItemEnum, ItemFn, ItemStruct};

use crate::compiler::analyses::call_graph::{
    ApplicationStateCallGraph, CallGraphNode, RawCallGraph,
};
use crate::compiler::analyses::components::ComponentDb;
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::framework_items::FrameworkItemDb;
use crate::compiler::analyses::processing_pipeline::{
    CodegenedRequestHandlerPipeline, RequestHandlerPipeline,
};
use crate::compiler::analyses::user_components::RouterKey;
use crate::compiler::app::GENERATED_APP_PACKAGE_ID;
use crate::compiler::computation::Computation;
use crate::language::{Callable, GenericArgument, ResolvedType};
use crate::rustdoc::{ALLOC_PACKAGE_ID_REPR, TOOLCHAIN_CRATES};

use super::generated_app::GeneratedManifest;

#[derive(Debug, Clone)]
enum CodegenRouterEntry {
    MethodSubRouter(BTreeMap<String, CodegenedRequestHandlerPipeline>),
    CatchAllHandler(CodegenedRequestHandlerPipeline),
}

pub(crate) fn codegen_app(
    handler_pipelines: &IndexMap<RouterKey, RequestHandlerPipeline>,
    application_state_call_graph: &ApplicationStateCallGraph,
    request_scoped_framework_bindings: &BiHashMap<Ident, ResolvedType>,
    package_id2name: &BiHashMap<PackageId, String>,
    runtime_singleton_bindings: &BiHashMap<Ident, ResolvedType>,
    codegen_deps: &HashMap<String, PackageId>,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    framework_item_db: &FrameworkItemDb,
) -> Result<TokenStream, anyhow::Error> {
    let get_codegen_dep_import_name = |name: &str| {
        let pkg_id = codegen_deps.get(name).unwrap();
        let import_name = package_id2name.get_by_left(pkg_id).unwrap().clone();
        format_ident!("{}", import_name)
    };
    let pavex_import_name = get_codegen_dep_import_name("pavex");
    let http_import_name = get_codegen_dep_import_name("http");
    let thiserror_import_name = get_codegen_dep_import_name("thiserror");

    let application_state_def =
        define_application_state(runtime_singleton_bindings, package_id2name);
    let define_application_state_error = define_application_state_error(
        &application_state_call_graph.error_variants,
        package_id2name,
        &thiserror_import_name,
    );
    let application_state_init = get_application_state_init(
        application_state_call_graph,
        package_id2name,
        component_db,
        computation_db,
    )?;

    let define_server_state = define_server_state(&application_state_def, &pavex_import_name);

    let mut handler_modules = vec![];
    let path2codegen_router_entry = {
        let mut map: IndexMap<String, CodegenRouterEntry> = IndexMap::new();
        for (router_key, pipeline) in handler_pipelines {
            let pipeline_code = pipeline.codegen(package_id2name, component_db, computation_db)?;
            handler_modules.push(pipeline_code.as_inline_module());
            match router_key.method_guard.clone() {
                None => {
                    map.insert(
                        router_key.path.clone(),
                        CodegenRouterEntry::CatchAllHandler(pipeline_code.clone()),
                    );
                }
                Some(methods) => {
                    let sub_router = map
                        .entry(router_key.path.clone())
                        .or_insert_with(|| CodegenRouterEntry::MethodSubRouter(BTreeMap::new()));
                    let CodegenRouterEntry::MethodSubRouter(sub_router) = sub_router else {
                        unreachable!("Cannot have a catch-all handler and a method sub-router for the same path");
                    };
                    for method in methods {
                        sub_router.insert(method, pipeline_code.clone());
                    }
                }
            }
        }
        map
    };

    let mut route_id2path = BiBTreeMap::new();
    let mut route_id2router_entry = BTreeMap::new();
    for (route_id, (path, router_entry)) in path2codegen_router_entry.iter().enumerate() {
        route_id2path.insert(route_id as u32, path.to_owned());
        route_id2router_entry.insert(route_id as u32, router_entry.to_owned());
    }

    let router_init = get_router_init(&route_id2path, &pavex_import_name);
    let route_request = get_request_dispatcher(
        &route_id2router_entry,
        &route_id2path,
        runtime_singleton_bindings,
        request_scoped_framework_bindings,
        framework_item_db,
        &pavex_import_name,
        &http_import_name,
    );
    let entrypoint = server_startup(&pavex_import_name);
    let alloc_rename = if package_id2name.contains_right(ALLOC_PACKAGE_ID_REPR) {
        // The fact that an item from `alloc` is used in the generated code does not imply
        // that we need to have an `alloc` import (e.g. it might not appear in function
        // signatures).
        // That's why we add `#[allow(unused_imports)]` to the `alloc` import.
        quote! {
            #[allow(unused_imports)]
            use std as alloc;
        }
    } else {
        quote! {}
    };
    let code = quote! {
        //! Do NOT edit this code.
        //! It was automatically generated by Pavex.
        //! All manual edits will be lost next time the code is generated.
        #alloc_rename
        #define_server_state
        #application_state_def
        #define_application_state_error
        #application_state_init
        #entrypoint
        #router_init
        #route_request
        #(#handler_modules)*
    };
    Ok(code)
}

fn server_startup(pavex: &Ident) -> ItemFn {
    syn::parse2(quote! {
        pub fn run(
            server_builder: #pavex::server::Server,
            application_state: ApplicationState
        ) -> #pavex::server::ServerHandle {
            let server_state = std::sync::Arc::new(ServerState {
                router: build_router(),
                application_state
            });
            server_builder.serve(route_request, server_state)
        }
    })
    .unwrap()
}

fn define_application_state(
    runtime_singletons: &BiHashMap<Ident, ResolvedType>,
    package_id2name: &BiHashMap<PackageId, String>,
) -> ItemStruct {
    let mut runtime_singletons = runtime_singletons
        .iter()
        .map(|(field_name, type_)| {
            let field_type = type_.syn_type(package_id2name);
            (field_name, field_type)
        })
        .collect::<Vec<_>>();
    // Sort the fields by name to ensure that the generated code is deterministic.
    runtime_singletons.sort_by_key(|(field_name, _)| field_name.to_string());

    let singleton_fields = runtime_singletons.iter().map(|(field_name, type_)| {
        quote! { #field_name: #type_ }
    });
    syn::parse2(quote! {
        pub struct ApplicationState {
            #(#singleton_fields),*
        }
    })
    .unwrap()
}

fn define_application_state_error(
    error_types: &IndexMap<String, ResolvedType>,
    package_id2name: &BiHashMap<PackageId, String>,
    thiserror_import_name: &Ident,
) -> Option<ItemEnum> {
    if error_types.is_empty() {
        return None;
    }
    let singleton_fields = error_types.iter().map(|(variant_name, type_)| {
        let variant_type = type_.syn_type(package_id2name);
        let variant_name = format_ident!("{}", variant_name);
        quote! {
            #[error(transparent)]
            #variant_name(#variant_type)
        }
    });
    Some(
        syn::parse2(quote! {
            #[derive(Debug, #thiserror_import_name::Error)]
            pub enum ApplicationStateError {
                #(#singleton_fields),*
            }
        })
        .unwrap(),
    )
}

fn define_server_state(
    application_state_def: &ItemStruct,
    pavex_import_name: &Ident,
) -> ItemStruct {
    let attribute = if application_state_def.fields.is_empty() {
        quote! {
            #[allow(dead_code)]
        }
    } else {
        quote! {}
    };
    syn::parse2(quote! {
        struct ServerState {
            router: #pavex_import_name::routing::Router<u32>,
            #attribute
            application_state: ApplicationState
        }
    })
    .unwrap()
}

fn get_application_state_init(
    application_state_call_graph: &ApplicationStateCallGraph,
    package_id2name: &BiHashMap<PackageId, String>,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
) -> Result<ItemFn, anyhow::Error> {
    let mut function = application_state_call_graph.call_graph.codegen(
        package_id2name,
        component_db,
        computation_db,
    )?;
    function.sig.ident = format_ident!("build_application_state");
    if !application_state_call_graph.error_variants.is_empty() {
        function.sig.output = syn::ReturnType::Type(
            Default::default(),
            Box::new(syn::parse2(
                quote! { Result<crate::ApplicationState, crate::ApplicationStateError> },
            )?),
        );
    }
    Ok(function)
}

fn get_router_init(route_id2path: &BiBTreeMap<u32, String>, pavex_import_name: &Ident) -> ItemFn {
    let mut router_init = quote! {
        let mut router = #pavex_import_name::routing::Router::new();
    };
    for (route_id, path) in route_id2path {
        router_init = quote! {
            #router_init
            router.insert(#path, #route_id).unwrap();
        };
    }
    syn::parse2(quote! {
        fn build_router() -> #pavex_import_name::routing::Router<u32> {
            // Pavex has validated at compile-time that all route paths are valid
            // and that there are no conflicts, therefore we can safely unwrap
            // every `insert`.
            #router_init
            router
        }
    })
    .unwrap()
}

fn get_request_dispatcher(
    route_id2router_entry: &BTreeMap<u32, CodegenRouterEntry>,
    route_id2path: &BiBTreeMap<u32, String>,
    singleton_bindings: &BiHashMap<Ident, ResolvedType>,
    request_scoped_bindings: &BiHashMap<Ident, ResolvedType>,
    framework_items_db: &FrameworkItemDb,
    pavex: &Ident,
    http: &Ident,
) -> ItemFn {
    let mut route_dispatch_table = quote! {};
    let server_state_ident = format_ident!("server_state");

    let matched_route_type = framework_items_db
        .get_type(FrameworkItemDb::matched_route_template_id())
        .unwrap();

    for (route_id, router_entry) in route_id2router_entry {
        let match_arm = match router_entry {
            CodegenRouterEntry::MethodSubRouter(sub_router) => {
                let mut sub_router_dispatch_table = quote! {};
                let mut allowed_methods = vec![];

                let needs_matched_route = sub_router.values().any(|pipeline| {
                    pipeline.stages[0]
                        .input_parameters
                        .contains(matched_route_type)
                });

                for (method, request_pipeline) in sub_router {
                    let invocation = request_pipeline.entrypoint_invocation(
                        singleton_bindings,
                        request_scoped_bindings,
                        &server_state_ident,
                    );
                    allowed_methods.push(method.clone());
                    let method = format_ident!("{}", method);
                    sub_router_dispatch_table = quote! {
                        #sub_router_dispatch_table
                        &#pavex::http::Method::#method => #invocation,
                    }
                }
                let allow_header_value = allowed_methods.join(", ");
                let matched_route_template = if needs_matched_route {
                    let path = route_id2path.get_by_left(route_id).unwrap();
                    quote! {
                        let matched_route_template = #pavex::extract::route::MatchedRouteTemplate::new(
                            #path
                        );
                    }
                } else {
                    quote! {}
                };
                quote! {
                    {
                    #matched_route_template
                    match &request_head.method {
                        #sub_router_dispatch_table
                        _ => {
                            let header_value = #pavex::http::HeaderValue::from_static(#allow_header_value);
                            #pavex::response::Response::method_not_allowed()
                                .insert_header(#pavex::http::header::ALLOW, header_value)
                                .box_body()
                        }
                    }
                    }
                }
            }
            CodegenRouterEntry::CatchAllHandler(request_pipeline) => request_pipeline
                .entrypoint_invocation(
                    singleton_bindings,
                    request_scoped_bindings,
                    &server_state_ident,
                ),
        };
        route_dispatch_table = quote! {
            #route_dispatch_table
            #route_id => #match_arm,
        };
    }

    syn::parse2(quote! {
        async fn route_request(
            request: #http::Request<#pavex::hyper::body::Incoming>,
            #server_state_ident: std::sync::Arc<ServerState>
        ) -> #pavex::response::Response {
            #[allow(unused)]
            let (request_head, request_body) = request.into_parts();
            let request_head: #pavex::request::RequestHead = request_head.into();
            let matched_route = match server_state.router.at(&request_head.uri.path()) {
                Ok(m) => m,
                Err(_) => {
                    return #pavex::response::Response::not_found().box_body();
                }
            };
            let route_id = matched_route.value;
            #[allow(unused)]
            let url_params: #pavex::extract::route::RawRouteParams<'_, '_> = matched_route
                .params
                .into();
            match route_id {
                #route_dispatch_table
                _ => #pavex::response::Response::not_found().box_body(),
            }
        }
    })
    .unwrap()
}

pub(crate) fn codegen_manifest<'a, I>(
    package_graph: &guppy::graph::PackageGraph,
    handler_call_graphs: I,
    application_state_call_graph: &'a RawCallGraph,
    request_scoped_framework_bindings: &'a BiHashMap<Ident, ResolvedType>,
    codegen_deps: &'a HashMap<String, PackageId>,
    component_db: &'a ComponentDb,
    computation_db: &'a ComputationDb,
) -> (GeneratedManifest, BiHashMap<PackageId, String>)
where
    I: Iterator<Item = &'a RequestHandlerPipeline>,
{
    let (dependencies, mut package_ids2deps) = compute_dependencies(
        package_graph,
        handler_call_graphs,
        application_state_call_graph,
        request_scoped_framework_bindings,
        codegen_deps,
        component_db,
        computation_db,
    );
    let manifest = GeneratedManifest {
        dependencies,
        edition: Edition::E2021,
    };

    // Toolchain crates are not listed as dependencies in the manifest, but we need to add them to
    // the package_ids2deps map so that we can generate the correct import statements.
    let toolchain_package_ids = TOOLCHAIN_CRATES
        .iter()
        .map(|p| PackageId::new(*p))
        .collect::<Vec<_>>();
    for package_id in &toolchain_package_ids {
        package_ids2deps.insert(package_id.clone(), package_id.repr().into());
    }

    // Same for the generated app package: local items can be imported using the `crate` shortcut.
    let generated_app_package_id = PackageId::new(GENERATED_APP_PACKAGE_ID);
    package_ids2deps.insert(generated_app_package_id, "crate".into());

    (manifest, package_ids2deps)
}

fn compute_dependencies<'a, I>(
    package_graph: &guppy::graph::PackageGraph,
    handler_pipelines: I,
    application_state_call_graph: &'a RawCallGraph,
    request_scoped_framework_bindings: &'a BiHashMap<Ident, ResolvedType>,
    codegen_deps: &'a HashMap<String, PackageId>,
    component_db: &'a ComponentDb,
    computation_db: &'a ComputationDb,
) -> (BTreeMap<String, Dependency>, BiHashMap<PackageId, String>)
where
    I: Iterator<Item = &'a RequestHandlerPipeline>,
{
    let package_ids = collect_package_ids(
        handler_pipelines,
        application_state_call_graph,
        request_scoped_framework_bindings,
        codegen_deps,
        component_db,
        computation_db,
    );
    let mut external_crates: IndexMap<&str, IndexSet<PackageId>> = Default::default();
    let workspace_root = package_graph.workspace().root();
    for package_id in &package_ids {
        if package_id.repr() != GENERATED_APP_PACKAGE_ID
            && !TOOLCHAIN_CRATES.contains(&package_id.repr())
        {
            let metadata = package_graph.metadata(package_id).unwrap();
            external_crates
                .entry(metadata.name())
                .or_default()
                .insert(package_id.to_owned());
        }
    }
    let mut dependencies = BTreeMap::new();
    let mut package_ids2dependency_name = BiHashMap::new();
    for (name, entries) in external_crates {
        let needs_rename = entries.len() > 1;
        for package_id in &entries {
            let metadata = package_graph.metadata(package_id).unwrap();
            let version = metadata.version();
            let mut dependency_details = DependencyDetail {
                package: Some(name.to_string()),
                version: Some(version.to_string()),
                ..DependencyDetail::default()
            };

            let source = metadata.source();
            match source {
                PackageSource::Workspace(p) | PackageSource::Path(p) => {
                    let path = if p.is_relative() {
                        workspace_root.join(p)
                    } else {
                        p.to_owned()
                    };
                    dependency_details.path = Some(path.to_string());
                }
                PackageSource::External(_) => {
                    if let Some(parsed_external) = source.parse_external() {
                        match parsed_external {
                            ExternalSource::Registry(registry) => {
                                if registry != ExternalSource::CRATES_IO_URL {
                                    // TODO: this is unlikely to work as is, because the `Cargo.toml` should contain
                                    //   the registry alias, not the raw registry URL.
                                    //   We can retrieve the alias from the .cargo/config.toml (probably).
                                    dependency_details.registry = Some(registry.to_string());
                                }
                            }
                            ExternalSource::Git {
                                repository, req, ..
                            } => {
                                dependency_details.git = Some(repository.to_string());
                                match req {
                                    guppy::graph::GitReq::Branch(branch) => {
                                        dependency_details.branch = Some(branch.to_string());
                                    }
                                    guppy::graph::GitReq::Tag(tag) => {
                                        dependency_details.tag = Some(tag.to_string());
                                    }
                                    guppy::graph::GitReq::Rev(rev) => {
                                        dependency_details.rev = Some(rev.to_string());
                                    }
                                    guppy::graph::GitReq::Default => {}
                                    _ => panic!("Unknown git requirements: {:?}", req),
                                }
                            }
                            _ => panic!("External source of unknown kind: {}", parsed_external),
                        }
                    } else {
                        panic!("Could not parse external source: {}", source);
                    }
                }
            }

            let dependency_name = if needs_rename {
                // TODO: this won't be unique if there are multiple versions of the same crate that have the same
                //   major/minor/patch version but differ in the pre-release version (e.g. `0.0.1-alpha` and `0.0.1-beta`).
                format!(
                    "{}_{}_{}_{}",
                    name, version.major, version.minor, version.patch
                )
            } else {
                name.to_string()
            }
            .replace('-', "_");

            dependencies.insert(
                dependency_name.clone(),
                Dependency::Detailed(dependency_details),
            );
            package_ids2dependency_name.insert(package_id.to_owned(), dependency_name);
        }
    }
    (dependencies, package_ids2dependency_name)
}

fn collect_package_ids<'a, I>(
    handler_pipelines: I,
    application_state_call_graph: &'a RawCallGraph,
    request_scoped_framework_bindings: &'a BiHashMap<Ident, ResolvedType>,
    codegen_deps: &'a HashMap<String, PackageId>,
    component_db: &'a ComponentDb,
    computation_db: &'a ComputationDb,
) -> IndexSet<PackageId>
where
    I: Iterator<Item = &'a RequestHandlerPipeline>,
{
    let mut package_ids = IndexSet::new();
    for t in request_scoped_framework_bindings.right_values() {
        collect_type_package_ids(&mut package_ids, t);
    }
    for package_id in codegen_deps.values() {
        package_ids.insert(package_id.to_owned());
    }
    collect_call_graph_package_ids(
        &mut package_ids,
        component_db,
        computation_db,
        application_state_call_graph,
    );
    for handler_pipeline in handler_pipelines {
        for graph in handler_pipeline.graph_iter() {
            collect_call_graph_package_ids(
                &mut package_ids,
                component_db,
                computation_db,
                &graph.call_graph,
            );
        }
    }
    package_ids
}

fn collect_call_graph_package_ids<'a>(
    package_ids: &mut IndexSet<PackageId>,
    component_db: &'a ComponentDb,
    computation_db: &'a ComputationDb,
    call_graph: &'a RawCallGraph,
) {
    for node in call_graph.node_weights() {
        match node {
            CallGraphNode::Compute { component_id, .. } => {
                let component = component_db.hydrated_component(*component_id, computation_db);
                match component.computation() {
                    Computation::Callable(c) => {
                        collect_callable_package_ids(package_ids, &c);
                    }
                    Computation::MatchResult(m) => {
                        collect_type_package_ids(package_ids, &m.input);
                        collect_type_package_ids(package_ids, &m.output);
                    }
                    Computation::FrameworkItem(i) => {
                        collect_type_package_ids(package_ids, &i);
                    }
                }
            }
            CallGraphNode::InputParameter { type_, .. } => {
                collect_type_package_ids(package_ids, type_)
            }
            CallGraphNode::MatchBranching => {}
        }
    }
}

fn collect_callable_package_ids(package_ids: &mut IndexSet<PackageId>, c: &Callable) {
    package_ids.insert(c.path.package_id.clone());
    for input in &c.inputs {
        collect_type_package_ids(package_ids, input);
    }
    if let Some(output) = c.output.as_ref() {
        collect_type_package_ids(package_ids, output);
    }
}

fn collect_type_package_ids(package_ids: &mut IndexSet<PackageId>, t: &ResolvedType) {
    match t {
        ResolvedType::ResolvedPath(t) => {
            package_ids.insert(t.package_id.clone());
            for generic in &t.generic_arguments {
                match generic {
                    GenericArgument::TypeParameter(t) => collect_type_package_ids(package_ids, t),
                    GenericArgument::Lifetime(_) => {}
                }
            }
        }
        ResolvedType::Reference(t) => collect_type_package_ids(package_ids, &t.inner),
        ResolvedType::Tuple(t) => {
            for element in &t.elements {
                collect_type_package_ids(package_ids, element)
            }
        }
        ResolvedType::Slice(s) => {
            collect_type_package_ids(package_ids, &s.element_type);
        }
        ResolvedType::Generic(_) | ResolvedType::ScalarPrimitive(_) => {}
    }
}
