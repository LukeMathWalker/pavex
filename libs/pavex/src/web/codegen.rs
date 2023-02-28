use std::collections::BTreeMap;
use std::path::PathBuf;

use ahash::HashSet;
use bimap::{BiBTreeMap, BiHashMap};
use cargo_manifest::{Dependency, DependencyDetail, Edition, MaybeInherited};
use guppy::graph::PackageSource;
use guppy::{PackageId, Version};
use indexmap::{IndexMap, IndexSet};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{ItemEnum, ItemFn, ItemStruct};

use crate::language::{Callable, GenericArgument, ResolvedType};
use crate::rustdoc::{ALLOC_PACKAGE_ID, TOOLCHAIN_CRATES};
use crate::web::analyses::call_graph::{ApplicationStateCallGraph, CallGraph, CallGraphNode};
use crate::web::analyses::components::{ComponentDb, HydratedComponent};
use crate::web::analyses::computations::ComputationDb;
use crate::web::analyses::user_components::RouterKey;
use crate::web::app::GENERATED_APP_PACKAGE_ID;
use crate::web::computation::Computation;
use crate::web::constructors::Constructor;

#[derive(Debug, Clone)]
enum CodegenRouterEntry {
    MethodSubRouter(BTreeMap<String, CodegenRequestHandler>),
    CatchAllHandler(CodegenRequestHandler),
}

#[derive(Debug, Clone)]
struct CodegenRequestHandler {
    code: ItemFn,
    input_types: IndexSet<ResolvedType>,
}

impl CodegenRequestHandler {
    fn invocation(
        &self,
        singleton_bindings: &BiHashMap<Ident, ResolvedType>,
        request_scoped_bindings: &BiHashMap<Ident, ResolvedType>,
        server_state_ident: &Ident,
    ) -> TokenStream {
        let handler = &self.code;
        let handler_input_types = &self.input_types;
        let is_handler_async = handler.sig.asyncness.is_some();
        let handler_function_name = &handler.sig.ident;
        let input_parameters = handler_input_types.iter().map(|type_| {
            let mut is_shared_reference = false;
            let inner_type = match type_ {
                ResolvedType::Reference(r) => {
                    if !r.is_static {
                        is_shared_reference = true;
                        &r.inner
                    } else {
                        type_
                    }
                }
                ResolvedType::Slice(_)
                | ResolvedType::ResolvedPath(_)
                | ResolvedType::Tuple(_)
                | ResolvedType::ScalarPrimitive(_) => type_,
            };
            if let Some(field_name) = singleton_bindings.get_by_right(inner_type) {
                if is_shared_reference {
                    quote! {
                        &#server_state_ident.application_state.#field_name
                    }
                } else {
                    quote! {
                        #server_state_ident.application_state.#field_name.clone()
                    }
                }
            } else if let Some(field_name) = request_scoped_bindings.get_by_right(type_) {
                quote! {
                    #field_name
                }
            } else {
                let field_name = request_scoped_bindings.get_by_right(inner_type).unwrap();
                quote! {
                    #field_name
                }
            }
        });
        let mut handler_invocation = quote! { #handler_function_name(#(#input_parameters),*) };
        if is_handler_async {
            handler_invocation = quote! { #handler_invocation.await };
        }
        handler_invocation
    }
}

pub(crate) fn codegen_app(
    handler_call_graphs: &IndexMap<RouterKey, CallGraph>,
    application_state_call_graph: &ApplicationStateCallGraph,
    request_scoped_framework_bindings: &BiHashMap<Ident, ResolvedType>,
    package_id2name: &BiHashMap<PackageId, String>,
    runtime_singleton_bindings: &BiHashMap<Ident, ResolvedType>,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
) -> Result<TokenStream, anyhow::Error> {
    let define_application_state =
        define_application_state(runtime_singleton_bindings, package_id2name);
    let define_application_state_error = define_application_state_error(
        &application_state_call_graph.error_variants,
        package_id2name,
    );
    let application_state_init = get_application_state_init(
        application_state_call_graph,
        package_id2name,
        component_db,
        computation_db,
    )?;
    let define_server_state = define_server_state();

    let mut handlers = vec![];
    let path2codegen_router_entry = {
        let mut map: IndexMap<String, CodegenRouterEntry> = IndexMap::new();
        for (i, (router_key, call_graph)) in handler_call_graphs.iter().enumerate() {
            let mut code = call_graph.codegen(package_id2name, component_db, computation_db)?;
            code.sig.ident = format_ident!("route_handler_{}", i);
            handlers.push(code.clone());
            let handler = CodegenRequestHandler {
                code,
                input_types: call_graph.required_input_types(),
            };
            match router_key.method_guard.clone() {
                None => {
                    map.insert(
                        router_key.path.clone(),
                        CodegenRouterEntry::CatchAllHandler(handler),
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
                        sub_router.insert(method, handler.clone());
                    }
                }
            }
        }
        map
    };

    // TODO: enforce that handlers have the right signature
    // TODO: enforce that the only required input is a Request type of some kind
    let mut route_id2path = BiBTreeMap::new();
    let mut route_id2router_entry = BTreeMap::new();
    for (route_id, (path, router_entry)) in path2codegen_router_entry.iter().enumerate() {
        route_id2path.insert(route_id as u32, path.to_owned());
        route_id2router_entry.insert(route_id as u32, router_entry.to_owned());
    }

    let router_init = get_router_init(&route_id2path);
    let route_request = get_request_dispatcher(
        &route_id2router_entry,
        runtime_singleton_bindings,
        request_scoped_framework_bindings,
    );
    let entrypoint = server_startup();
    let alloc_rename = if package_id2name.contains_right(ALLOC_PACKAGE_ID) {
        quote! { use std as alloc; }
    } else {
        quote! {}
    };
    let code = quote! {
        //! Do NOT edit this code.
        //! It was automatically generated by `pavex`.
        //! All manual edits will be lost next time the code is generated.
        #alloc_rename
        #define_server_state
        #define_application_state
        #define_application_state_error
        #application_state_init
        #entrypoint
        #router_init
        #route_request
        #(#handlers)*
    };
    Ok(code)
}

fn server_startup() -> ItemFn {
    syn::parse2(quote! {
        pub async fn run(
            server_builder: pavex_runtime::hyper::server::Builder<pavex_runtime::hyper::server::conn::AddrIncoming>,
            application_state: ApplicationState
        ) -> Result<(), pavex_runtime::Error> {
            let server_state = std::sync::Arc::new(ServerState {
                router: build_router().map_err(pavex_runtime::Error::new)?,
                application_state
            });
            let make_service = pavex_runtime::hyper::service::make_service_fn(move |_| {
                let server_state = server_state.clone();
                async move {
                    Ok::<_, pavex_runtime::hyper::Error>(pavex_runtime::hyper::service::service_fn(move |request| {
                        let server_state = server_state.clone();
                        async move { Ok::<_, pavex_runtime::hyper::Error>(route_request(request, server_state).await) }
                    }))
                }
            });
            server_builder.serve(make_service).await.map_err(pavex_runtime::Error::new)
        }
    }).unwrap()
}

fn define_application_state(
    runtime_singletons: &BiHashMap<Ident, ResolvedType>,
    package_id2name: &BiHashMap<PackageId, String>,
) -> ItemStruct {
    let singleton_fields = runtime_singletons.iter().map(|(field_name, type_)| {
        let field_type = type_.syn_type(package_id2name);
        quote! { #field_name: #field_type }
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
) -> Option<ItemEnum> {
    if error_types.is_empty() {
        return None;
    }
    let singleton_fields = error_types.iter().map(|(variant_name, type_)| {
        let variant_type = type_.syn_type(package_id2name);
        let variant_name = format_ident!("{}", variant_name);
        quote! { #variant_name(#variant_type) }
    });
    // TODO: implement `Display` + `Error` for `ApplicationStateError`
    Some(
        syn::parse2(quote! {
            #[derive(Debug)]
            pub enum ApplicationStateError {
                #(#singleton_fields),*
            }
        })
        .unwrap(),
    )
}

fn define_server_state() -> ItemStruct {
    syn::parse2(quote! {
        struct ServerState {
            router: pavex_runtime::routing::Router<u32>,
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

fn get_router_init(route_id2path: &BiBTreeMap<u32, String>) -> ItemFn {
    let mut router_init = quote! {
        let mut router = pavex_runtime::routing::Router::new();
    };
    for (route_id, path) in route_id2path {
        router_init = quote! {
            #router_init
            router.insert(#path, #route_id)?;
        };
    }
    syn::parse2(quote! {
        fn build_router() -> Result<pavex_runtime::routing::Router<u32>, pavex_runtime::routing::InsertError> {
            #router_init
            Ok(router)
        }
    }).unwrap()
}

fn get_request_dispatcher(
    route_id2router_entry: &BTreeMap<u32, CodegenRouterEntry>,
    singleton_bindings: &BiHashMap<Ident, ResolvedType>,
    request_scoped_bindings: &BiHashMap<Ident, ResolvedType>,
) -> ItemFn {
    let mut route_dispatch_table = quote! {};
    let server_state_ident = format_ident!("server_state");

    for (route_id, router_entry) in route_id2router_entry {
        let match_arm = match router_entry {
            CodegenRouterEntry::MethodSubRouter(sub_router) => {
                let mut sub_router_dispatch_table = quote! {};
                let mut allowed_methods = vec![];
                for (method, handler) in sub_router {
                    let invocation = handler.invocation(
                        singleton_bindings,
                        request_scoped_bindings,
                        &server_state_ident,
                    );
                    allowed_methods.push(method.clone());
                    let method = format_ident!("{}", method);
                    sub_router_dispatch_table = quote! {
                        #sub_router_dispatch_table
                        &pavex_runtime::http::Method::#method => #invocation,
                    }
                }
                let allow_header_value = allowed_methods.join(", ");
                quote! {
                    match request.method() {
                        #sub_router_dispatch_table
                        _ => {
                            pavex_runtime::response::Response::builder()
                                .status(pavex_runtime::http::StatusCode::METHOD_NOT_ALLOWED)
                                .header(pavex_runtime::http::header::ALLOW, #allow_header_value)
                                .body(pavex_runtime::body::boxed(hyper::body::Body::empty()))
                                .unwrap()
                        }
                    }
                }
            }
            CodegenRouterEntry::CatchAllHandler(h) => h.invocation(
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
        async fn route_request(request: pavex_runtime::http::Request<pavex_runtime::hyper::body::Body>, #server_state_ident: std::sync::Arc<ServerState>) -> pavex_runtime::response::Response {
            let route_id = server_state.router.at(request.uri().path()).expect("Failed to match incoming request path");
            match route_id.value {
                #route_dispatch_table
                _ => {
                    pavex_runtime::response::Response::builder()
                        .status(pavex_runtime::http::StatusCode::NOT_FOUND)
                        .body(pavex_runtime::body::boxed(hyper::body::Body::empty()))
                        .unwrap()
                }
            }
        }
    }).unwrap()
}

pub(crate) fn codegen_manifest<'a>(
    package_graph: &guppy::graph::PackageGraph,
    handler_call_graphs: &'a IndexMap<RouterKey, CallGraph>,
    application_state_call_graph: &'a CallGraph,
    request_scoped_framework_bindings: &'a BiHashMap<Ident, ResolvedType>,
    codegen_types: &'a HashSet<ResolvedType>,
    component_db: &'a ComponentDb,
    computation_db: &'a ComputationDb,
) -> (cargo_manifest::Manifest, BiHashMap<PackageId, String>) {
    let (dependencies, package_ids2deps) = compute_dependencies(
        package_graph,
        handler_call_graphs,
        application_state_call_graph,
        request_scoped_framework_bindings,
        codegen_types,
        component_db,
        computation_db,
    );
    let manifest = cargo_manifest::Manifest {
        dependencies: Some(dependencies),
        cargo_features: None,
        package: Some(cargo_manifest::Package {
            // TODO: this should be configurable
            name: "application".to_string(),
            edition: Some(MaybeInherited::Local(Edition::E2021)),
            version: MaybeInherited::Local("0.1.0".to_string()),
            build: None,
            workspace: None,
            authors: None,
            links: None,
            description: None,
            homepage: None,
            documentation: None,
            readme: None,
            keywords: None,
            categories: None,
            license: None,
            license_file: None,
            repository: None,
            metadata: None,
            rust_version: None,
            exclude: None,
            include: None,
            default_run: None,
            autobins: false,
            autoexamples: false,
            autotests: false,
            autobenches: false,
            publish: Default::default(),
            resolver: None,
        }),
        workspace: None,
        dev_dependencies: None,
        build_dependencies: None,
        target: None,
        features: None,
        bin: None,
        bench: None,
        test: None,
        example: None,
        patch: None,
        lib: None,
        profile: None,
        badges: None,
    };
    (manifest, package_ids2deps)
}

fn compute_dependencies<'a>(
    package_graph: &guppy::graph::PackageGraph,
    handler_call_graphs: &'a IndexMap<RouterKey, CallGraph>,
    application_state_call_graph: &'a CallGraph,
    request_scoped_framework_bindings: &'a BiHashMap<Ident, ResolvedType>,
    codegen_types: &'a HashSet<ResolvedType>,
    component_db: &'a ComponentDb,
    computation_db: &'a ComputationDb,
) -> (BTreeMap<String, Dependency>, BiHashMap<PackageId, String>) {
    let package_ids = collect_package_ids(
        handler_call_graphs,
        application_state_call_graph,
        request_scoped_framework_bindings,
        codegen_types,
        component_db,
        computation_db,
    );
    #[allow(clippy::type_complexity)]
    let mut external_crates: IndexMap<&str, IndexSet<(&Version, PackageId, Option<PathBuf>)>> =
        Default::default();
    let workspace_root = package_graph.workspace().root();
    for package_id in &package_ids {
        if package_id.repr() != GENERATED_APP_PACKAGE_ID
            && !TOOLCHAIN_CRATES.contains(&package_id.repr())
        {
            let metadata = package_graph.metadata(package_id).unwrap();
            let path = match metadata.source() {
                PackageSource::Workspace(p) | PackageSource::Path(p) => {
                    let path = if p.is_relative() {
                        workspace_root.join(p)
                    } else {
                        p.to_owned()
                    };
                    Some(path.into_std_path_buf())
                }
                // TODO: handle external deps
                PackageSource::External(_) => None,
            };
            external_crates.entry(metadata.name()).or_default().insert((
                metadata.version(),
                package_id.to_owned(),
                path,
            ));
        }
    }
    let mut dependencies = BTreeMap::new();
    let mut package_ids2dependency_name = BiHashMap::new();
    for (name, versions) in external_crates {
        if versions.len() == 1 {
            let (version, package_id, path) = versions.into_iter().next().unwrap();
            let dependency = if let Some(path) = path {
                cargo_manifest::Dependency::Detailed(DependencyDetail {
                    package: Some(name.to_string()),
                    version: Some(version.to_string()),
                    path: Some(path.to_string_lossy().to_string()),
                    ..DependencyDetail::default()
                })
            } else {
                cargo_manifest::Dependency::Simple(version.to_string())
            };
            dependencies.insert(name.to_owned(), dependency);
            package_ids2dependency_name.insert(package_id, name.replace('-', "_"));
        } else {
            for (i, (version, package_id, path)) in versions.into_iter().enumerate() {
                let rename = format!("{}_{i}", name.replace('-', "_"));
                let dependency = cargo_manifest::Dependency::Detailed(DependencyDetail {
                    package: Some(name.to_string()),
                    version: Some(version.to_string()),
                    path: path.map(|p| p.to_string_lossy().to_string()),
                    ..DependencyDetail::default()
                });
                dependencies.insert(rename.clone(), dependency);
                package_ids2dependency_name.insert(package_id, rename);
            }
        }
    }
    (dependencies, package_ids2dependency_name)
}

fn collect_package_ids<'a>(
    handler_call_graphs: &'a IndexMap<RouterKey, CallGraph>,
    application_state_call_graph: &'a CallGraph,
    request_scoped_framework_bindings: &'a BiHashMap<Ident, ResolvedType>,
    codegen_types: &'a HashSet<ResolvedType>,
    component_db: &'a ComponentDb,
    computation_db: &'a ComputationDb,
) -> IndexSet<PackageId> {
    let mut package_ids = IndexSet::new();
    for t in request_scoped_framework_bindings.right_values() {
        collect_type_package_ids(&mut package_ids, t);
    }
    for t in codegen_types {
        collect_type_package_ids(&mut package_ids, t);
    }
    collect_call_graph_package_ids(
        &mut package_ids,
        component_db,
        computation_db,
        application_state_call_graph,
    );
    for handler_call_graph in handler_call_graphs.values() {
        collect_call_graph_package_ids(
            &mut package_ids,
            component_db,
            computation_db,
            handler_call_graph,
        );
    }
    package_ids
}

fn collect_call_graph_package_ids<'a>(
    package_ids: &mut IndexSet<PackageId>,
    component_db: &'a ComponentDb,
    computation_db: &'a ComputationDb,
    call_graph: &'a CallGraph,
) {
    for node in call_graph.call_graph.node_weights() {
        match node {
            CallGraphNode::Compute { component_id, .. } => {
                let component = component_db.hydrated_component(*component_id, computation_db);
                match component {
                    HydratedComponent::Transformer(Computation::BorrowSharedReference(b))
                    | HydratedComponent::Constructor(Constructor(
                        Computation::BorrowSharedReference(b),
                    )) => collect_type_package_ids(package_ids, &b.input),
                    HydratedComponent::Transformer(Computation::Callable(c))
                    | HydratedComponent::Constructor(Constructor(Computation::Callable(c))) => {
                        collect_callable_package_ids(package_ids, &c);
                    }
                    HydratedComponent::Transformer(Computation::MatchResult(m))
                    | HydratedComponent::Constructor(Constructor(Computation::MatchResult(m))) => {
                        collect_type_package_ids(package_ids, &m.input);
                        collect_type_package_ids(package_ids, &m.output);
                    }
                    HydratedComponent::RequestHandler(r) => {
                        collect_callable_package_ids(package_ids, &r.callable);
                    }
                    HydratedComponent::ErrorHandler(e) => {
                        collect_callable_package_ids(package_ids, &e.callable)
                    }
                }
            }
            CallGraphNode::InputParameter(t) => collect_type_package_ids(package_ids, t),
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
                    GenericArgument::AssignedTypeParameter(t) => {
                        collect_type_package_ids(package_ids, t)
                    }
                    GenericArgument::Lifetime(_) | GenericArgument::UnassignedTypeParameter(_) => {}
                }
            }
        }
        ResolvedType::Reference(t) => collect_type_package_ids(package_ids, &t.inner),
        ResolvedType::Tuple(t) => {
            for element in &t.elements {
                collect_type_package_ids(package_ids, element)
            }
        }
        ResolvedType::ScalarPrimitive(_) => {}
        ResolvedType::Slice(s) => {
            collect_type_package_ids(package_ids, &s.element_type);
        }
    }
}
