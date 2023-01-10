use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use bimap::{BiBTreeMap, BiHashMap};
use cargo_manifest::{Dependency, DependencyDetail, Edition, MaybeInherited};
use guppy::graph::PackageSource;
use guppy::{PackageId, Version};
use indexmap::{IndexMap, IndexSet};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{ItemEnum, ItemFn, ItemStruct};

use crate::language::{Callable, ResolvedType};
use crate::rustdoc::TOOLCHAIN_CRATES;
use crate::web::app::GENERATED_APP_PACKAGE_ID;
use crate::web::call_graph::{
    ApplicationStateCallGraph, CallGraph, CallGraphNode, ComputeComponent,
};
use crate::web::constructors::Constructor;

pub(crate) fn codegen_app(
    handler_call_graphs: &IndexMap<String, CallGraph>,
    application_state_call_graph: &ApplicationStateCallGraph,
    request_scoped_framework_bindings: &BiHashMap<Ident, ResolvedType>,
    package_id2name: &BiHashMap<&'_ PackageId, String>,
    runtime_singleton_bindings: &BiHashMap<Ident, ResolvedType>,
) -> Result<TokenStream, anyhow::Error> {
    let define_application_state =
        define_application_state(runtime_singleton_bindings, package_id2name);
    let define_application_state_error =
        define_application_state_error(&application_state_call_graph.error_types, package_id2name);
    let application_state_init =
        get_application_state_init(&application_state_call_graph, package_id2name)?;
    let define_server_state = define_server_state();

    let handler_functions: IndexMap<_, _> = handler_call_graphs
        .into_iter()
        .map(|(path, call_graph)| {
            let code = call_graph.codegen(package_id2name)?;
            Ok::<_, anyhow::Error>((path, (code, call_graph.required_input_types())))
        })
        // TODO: wasteful
        .collect::<Result<IndexMap<_, _>, _>>()?
        .into_iter()
        .enumerate()
        .map(|(i, (path, (mut function, parameter_bindings)))| {
            // Ensure that all handler functions have a unique name.
            function.sig.ident = format_ident!("route_handler_{}", i);
            (path, (function, parameter_bindings))
        })
        .collect();

    // TODO: enforce that handlers have the right signature
    // TODO: enforce that the only required input is a Request type of some kind
    let mut route_id2path = BiBTreeMap::new();
    let mut route_id2handler = BTreeMap::new();
    for (route_id, (&route, handler)) in handler_functions.iter().enumerate() {
        route_id2path.insert(route_id as u32, route.to_owned());
        route_id2handler.insert(route_id as u32, handler.to_owned());
    }

    let router_init = get_router_init(&route_id2path);
    let route_request = get_request_dispatcher(
        &route_id2handler,
        runtime_singleton_bindings,
        request_scoped_framework_bindings,
    );
    let handlers = handler_functions.values().map(|(function, _)| function);
    let entrypoint = server_startup();
    let code = quote! {
        //! Do NOT edit this code.
        //! It was automatically generated by `pavex`.
        //! All manual edits will be lost next time the code is generated.
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
    package_id2name: &BiHashMap<&'_ PackageId, String>,
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
    error_types: &IndexSet<ResolvedType>,
    package_id2name: &BiHashMap<&'_ PackageId, String>,
) -> Option<ItemEnum> {
    if error_types.is_empty() {
        return None;
    }
    let singleton_fields = error_types.iter().map(|type_| {
        let variant_type = type_.syn_type(package_id2name);
        let variant_name = format_ident!("{}", type_.base_type.last().unwrap());
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
    package_id2name: &BiHashMap<&'_ PackageId, String>,
) -> Result<ItemFn, anyhow::Error> {
    let mut function = application_state_call_graph
        .call_graph
        .codegen(package_id2name)?;
    function.sig.ident = format_ident!("build_application_state");
    if !application_state_call_graph.error_types.is_empty() {
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
    route_id2handler: &BTreeMap<u32, (ItemFn, IndexSet<ResolvedType>)>,
    singleton_bindings: &BiHashMap<Ident, ResolvedType>,
    request_scoped_bindings: &BiHashMap<Ident, ResolvedType>,
) -> ItemFn {
    let mut route_dispatch_table = quote! {};

    for (route_id, (handler, handler_input_types)) in route_id2handler {
        let is_handler_async = handler.sig.asyncness.is_some();
        let handler_function_name = &handler.sig.ident;
        let input_parameters = handler_input_types.iter().map(|type_| {
            let is_shared_reference = type_.is_shared_reference;
            let inner_type = ResolvedType {
                is_shared_reference: false,
                ..type_.clone()
            };
            if let Some(field_name) = singleton_bindings.get_by_right(&inner_type) {
                if is_shared_reference {
                    quote! {
                        &server_state.application_state.#field_name
                    }
                } else {
                    quote! {
                        server_state.application_state.#field_name.clone()
                    }
                }
            } else if let Some(field_name) = request_scoped_bindings.get_by_right(type_) {
                quote! {
                    #field_name
                }
            } else {
                let field_name = request_scoped_bindings.get_by_right(&inner_type).unwrap();
                quote! {
                    #field_name
                }
            }
        });
        let mut handler_invocation = quote! { #handler_function_name(#(#input_parameters),*) };
        if is_handler_async {
            handler_invocation = quote! { #handler_invocation.await };
        }
        route_dispatch_table = quote! {
            #route_dispatch_table
            #route_id => #handler_invocation,
        }
    }

    syn::parse2(quote! {
        async fn route_request(request: pavex_runtime::http::Request<pavex_runtime::hyper::body::Body>, server_state: std::sync::Arc<ServerState>) -> pavex_runtime::response::Response {
            let route_id = server_state.router.at(request.uri().path()).expect("Failed to match incoming request path");
            match route_id.value {
                #route_dispatch_table
                _ => panic!("This is a bug, no route registered for a route id"),
            }
        }
    }).unwrap()
}

pub(crate) fn codegen_manifest<'a>(
    package_graph: &guppy::graph::PackageGraph,
    handler_call_graphs: &'a IndexMap<String, CallGraph>,
    application_state_call_graph: &'a CallGraph,
    request_scoped_framework_bindings: &'a BiHashMap<Ident, ResolvedType>,
    codegen_types: &'a HashSet<ResolvedType>,
) -> (cargo_manifest::Manifest, BiHashMap<&'a PackageId, String>) {
    let (dependencies, package_ids2deps) = compute_dependencies(
        package_graph,
        handler_call_graphs,
        application_state_call_graph,
        request_scoped_framework_bindings,
        codegen_types,
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
    handler_call_graphs: &'a IndexMap<String, CallGraph>,
    application_state_call_graph: &'a CallGraph,
    request_scoped_framework_bindings: &'a BiHashMap<Ident, ResolvedType>,
    codegen_types: &'a HashSet<ResolvedType>,
) -> (
    BTreeMap<String, Dependency>,
    BiHashMap<&'a PackageId, String>,
) {
    let package_ids = collect_package_ids(
        handler_call_graphs,
        application_state_call_graph,
        request_scoped_framework_bindings,
        codegen_types,
    );
    #[allow(clippy::type_complexity)]
    let mut external_crates: IndexMap<&str, IndexSet<(&Version, &PackageId, Option<PathBuf>)>> =
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
                package_id,
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
            package_ids2dependency_name.insert(package_id, name.to_owned());
        } else {
            for (i, (version, package_id, path)) in versions.into_iter().enumerate() {
                let rename = format!("{name}_{i}");
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
    handler_call_graphs: &'a IndexMap<String, CallGraph>,
    application_state_call_graph: &'a CallGraph,
    request_scoped_framework_bindings: &'a BiHashMap<Ident, ResolvedType>,
    codegen_types: &'a HashSet<ResolvedType>,
) -> IndexSet<&'a PackageId> {
    let mut package_ids = IndexSet::new();
    for t in request_scoped_framework_bindings.right_values() {
        package_ids.insert(&t.package_id);
    }
    for t in codegen_types {
        package_ids.insert(&t.package_id);
    }
    collect_call_graph_package_ids(&mut package_ids, application_state_call_graph);
    for handler_call_graph in handler_call_graphs.values() {
        collect_call_graph_package_ids(&mut package_ids, handler_call_graph);
    }
    package_ids
}

fn collect_call_graph_package_ids<'a>(
    package_ids: &mut IndexSet<&'a PackageId>,
    call_graph: &'a CallGraph,
) {
    for node in call_graph.call_graph.node_weights() {
        match node {
            CallGraphNode::Compute { component, .. } => match component {
                ComputeComponent::Constructor(c) => match c {
                    Constructor::BorrowSharedReference(t) => {
                        collect_type_package_ids(package_ids, &t.input)
                    }
                    Constructor::Callable(c) => {
                        collect_callable_package_ids(package_ids, c);
                    }
                    Constructor::MatchResult(m) => {
                        collect_type_package_ids(package_ids, &m.input);
                        collect_type_package_ids(package_ids, &m.output);
                    }
                },
                ComputeComponent::ErrorHandler(e) => {
                    collect_callable_package_ids(package_ids, e.as_ref())
                }
                ComputeComponent::Transformer(t) => collect_callable_package_ids(package_ids, t),
            },
            CallGraphNode::InputParameter(t) => collect_type_package_ids(package_ids, t),
            CallGraphNode::MatchBranching => {}
        }
    }
}

fn collect_callable_package_ids<'a>(package_ids: &mut IndexSet<&'a PackageId>, c: &'a Callable) {
    package_ids.insert(&c.path.package_id);
    for input in &c.inputs {
        collect_type_package_ids(package_ids, input);
    }
    if let Some(output) = c.output.as_ref() {
        collect_type_package_ids(package_ids, output);
    }
}

fn collect_type_package_ids<'a>(package_ids: &mut IndexSet<&'a PackageId>, t: &'a ResolvedType) {
    package_ids.insert(&t.package_id);
    for generic in &t.generic_arguments {
        collect_type_package_ids(package_ids, generic);
    }
}
