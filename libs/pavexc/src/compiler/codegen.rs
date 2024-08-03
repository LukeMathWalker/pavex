use std::collections::{BTreeMap, BTreeSet};

use ahash::{HashMap, HashSet};
use bimap::{BiBTreeMap, BiHashMap};
use cargo_manifest::{Dependency, DependencyDetail, Edition};
use guppy::graph::{ExternalSource, PackageSource};
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};
use once_cell::sync::Lazy;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{ItemEnum, ItemFn, ItemStruct};

use crate::compiler::analyses::call_graph::{
    ApplicationStateCallGraph, CallGraphNode, RawCallGraph,
};
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::framework_items::FrameworkItemDb;
use crate::compiler::analyses::processing_pipeline::{
    CodegenedRequestHandlerPipeline, RequestHandlerPipeline,
};
use crate::compiler::analyses::router::Router;
use crate::compiler::app::GENERATED_APP_PACKAGE_ID;
use crate::compiler::computation::Computation;
use crate::language::{Callable, GenericArgument, ResolvedType};
use crate::rustdoc::{ALLOC_PACKAGE_ID_REPR, TOOLCHAIN_CRATES};

use super::generated_app::GeneratedManifest;

#[derive(Debug, Clone)]
pub(super) struct CodegenMethodRouter {
    pub(super) methods_and_pipelines: Vec<(BTreeSet<String>, CodegenedRequestHandlerPipeline)>,
    pub(super) catch_all_pipeline: CodegenedRequestHandlerPipeline,
}

impl CodegenMethodRouter {
    pub fn pipelines(&self) -> impl Iterator<Item = &CodegenedRequestHandlerPipeline> {
        self.methods_and_pipelines
            .iter()
            .map(|(_, p)| p)
            .chain(std::iter::once(&self.catch_all_pipeline))
    }

    /// Returns `true` if any of the pipelines in this router needs `MatchedPathPattern`
    /// as input type.
    pub fn needs_matched_route(&self, framework_items_db: &FrameworkItemDb) -> bool {
        let matched_route_type = framework_items_db
            .get_type(FrameworkItemDb::matched_route_template_id())
            .unwrap();
        self.pipelines()
            .any(|pipeline| pipeline.needs_input_type(matched_route_type))
    }
}

pub(crate) fn codegen_app(
    router: &Router,
    handler_id2pipeline: &IndexMap<ComponentId, RequestHandlerPipeline>,
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
    let hyper_import_name = get_codegen_dep_import_name("hyper");
    let thiserror_import_name = get_codegen_dep_import_name("thiserror");
    let matchit_import_name = get_codegen_dep_import_name("matchit");

    let application_state_def =
        define_application_state(runtime_singleton_bindings, package_id2name);
    if tracing::event_enabled!(tracing::Level::TRACE) {
        eprintln!(
            "Application state definition:\n{:#?}",
            application_state_def
        );
    }
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

    let define_server_state = define_server_state(&application_state_def, &matchit_import_name);

    let handler_id2codegened_pipeline = handler_id2pipeline
        .iter()
        .map(|(id, p)| {
            let span = tracing::info_span!("Codegen request handler pipeline", route_info = %router.handler_id2route_info[id]);
            let _guard = span.enter();
            p.codegen(
                &pavex_import_name,
                package_id2name,
                component_db,
                computation_db,
            )
            .map(|p| (*id, p))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let handler_modules = handler_id2codegened_pipeline
        .values()
        .map(|p| p.as_inline_module())
        .collect::<Vec<_>>();
    let path2codegen_router_entry = {
        let mut map: IndexMap<String, CodegenMethodRouter> = IndexMap::new();
        for (path, method_router) in &router.route_path2sub_router {
            let mut methods_and_pipelines =
                Vec::with_capacity(method_router.handler_id2methods.len());
            for (handler_id, methods) in &method_router.handler_id2methods {
                let pipeline = &handler_id2codegened_pipeline[handler_id];
                methods_and_pipelines.push((methods.clone(), pipeline.clone()));
            }
            let catch_all_pipeline =
                handler_id2codegened_pipeline[&method_router.fallback_id].clone();
            map.insert(
                path.to_owned(),
                CodegenMethodRouter {
                    methods_and_pipelines,
                    catch_all_pipeline,
                },
            );
        }
        map
    };

    let mut route_id2path = BiBTreeMap::new();
    let mut route_id2router_entry = BTreeMap::new();
    for (route_id, (path, router_entry)) in path2codegen_router_entry.iter().enumerate() {
        route_id2path.insert(route_id as u32, path.to_owned());
        route_id2router_entry.insert(route_id as u32, router_entry.to_owned());
    }

    let router_init = get_router_init(&route_id2path, &matchit_import_name);
    let fallback_codegened_pipeline = &handler_id2codegened_pipeline[&router.root_fallback_id];
    let route_request = get_request_dispatcher(
        &route_id2router_entry,
        &route_id2path,
        fallback_codegened_pipeline,
        runtime_singleton_bindings,
        request_scoped_framework_bindings,
        framework_item_db,
        &pavex_import_name,
        &http_import_name,
        &hyper_import_name,
    );
    let entrypoint = server_startup(&pavex_import_name);
    let alloc_extern_import = if package_id2name.contains_right(ALLOC_PACKAGE_ID_REPR) {
        // The fact that an item from `alloc` is used in the generated code does not imply
        // that we need to have an `alloc` import (e.g. it might not appear in function
        // signatures).
        // Nonetheless, we add the import to be on the safe side.
        // See https://doc.rust-lang.org/edition-guide/rust-2018/path-changes.html#an-exception
        // for an explanation of why we need the "extern crate" syntax here.
        quote! {
            extern crate alloc;
        }
    } else {
        quote! {}
    };
    let code = quote! {
        //! Do NOT edit this code.
        //! It was automatically generated by Pavex.
        //! All manual edits will be lost next time the code is generated.
        #alloc_extern_import
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
    matchit_import_name: &Ident,
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
            router: #matchit_import_name::Router<u32>,
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

fn get_router_init(route_id2path: &BiBTreeMap<u32, String>, matchit_import_name: &Ident) -> ItemFn {
    let mut router_init = quote! {
        let mut router = #matchit_import_name::Router::new();
    };
    for (route_id, path) in route_id2path {
        router_init = quote! {
            #router_init
            router.insert(#path, #route_id).unwrap();
        };
    }
    syn::parse2(quote! {
        fn build_router() -> #matchit_import_name::Router<u32> {
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
    route_id2router_entry: &BTreeMap<u32, CodegenMethodRouter>,
    route_id2path: &BiBTreeMap<u32, String>,
    fallback_codegened_pipeline: &CodegenedRequestHandlerPipeline,
    singleton_bindings: &BiHashMap<Ident, ResolvedType>,
    request_scoped_bindings: &BiHashMap<Ident, ResolvedType>,
    framework_items_db: &FrameworkItemDb,
    pavex: &Ident,
    http: &Ident,
    hyper: &Ident,
) -> ItemFn {
    static WELL_KNOWN_METHODS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
        HashSet::from_iter(
            [
                "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "CONNECT", "TRACE",
            ]
            .into_iter(),
        )
    });

    let mut needs_connection_info = false;
    let mut route_dispatch_table = quote! {};
    let server_state_ident = format_ident!("server_state");

    for (route_id, sub_router) in route_id2router_entry {
        let match_arm = if sub_router.methods_and_pipelines.is_empty() {
            // We just have the catch-all handler, we can skip the `match`.
            sub_router.catch_all_pipeline.entrypoint_invocation(
                singleton_bindings,
                request_scoped_bindings,
                &server_state_ident,
            )
        } else {
            let mut sub_router_dispatch_table = quote! {};
            let allowed_methods_init = {
                let allowed_methods = sub_router
                    .methods_and_pipelines
                    .iter()
                    .flat_map(|(methods, _)| methods)
                    .map(|m| {
                        if WELL_KNOWN_METHODS.contains(m.as_str()) {
                            let i = format_ident!("{}", m);
                            quote! {
                                #pavex::http::Method::#i
                            }
                        } else {
                            let expect_msg = format!("{} is not a valid (custom) HTTP method", m);
                            quote! {
                                #pavex::http::Method::try_from(#m).expect(#expect_msg)
                            }
                        }
                    });
                quote! {
                    let allowed_methods: #pavex::router::AllowedMethods = #pavex::router::MethodAllowList::from_iter(
                        [#(#allowed_methods),*]
                    ).into();
                }
            };

            for (methods, request_pipeline) in &sub_router.methods_and_pipelines {
                let invocation = request_pipeline.entrypoint_invocation(
                    singleton_bindings,
                    request_scoped_bindings,
                    &server_state_ident,
                );
                let invocation = if request_pipeline.needs_allowed_methods(framework_items_db) {
                    quote! {
                        {
                            #allowed_methods_init
                            #invocation
                        }
                    }
                } else {
                    invocation
                };

                let invocation = if request_pipeline.needs_connection_info(framework_items_db) {
                    needs_connection_info = true;
                    quote! {
                        {
                            let connection_info = connection_info.expect("Required ConnectionInfo is missing");
                            #invocation
                        }
                    }
                } else {
                    invocation
                };

                let (well_known_methods, custom_methods) = methods
                    .iter()
                    .partition::<Vec<_>, _>(|m| WELL_KNOWN_METHODS.contains(m.as_str()));

                if !well_known_methods.is_empty() {
                    let well_known_methods = well_known_methods.into_iter().map(|m| {
                        let m = format_ident!("{}", m);
                        quote! {
                            #pavex::http::Method::#m
                        }
                    });

                    sub_router_dispatch_table = quote! {
                        #sub_router_dispatch_table
                        #(&#well_known_methods)|* => #invocation,
                    };
                };

                if !custom_methods.is_empty() {
                    let custom_methods = custom_methods.into_iter().map(|m| {
                        quote! {
                            s.as_str() == #m
                        }
                    });
                    sub_router_dispatch_table = quote! {
                        #sub_router_dispatch_table
                        s if #(#custom_methods)||* => #invocation,
                    };
                };
            }
            let matched_route_template = if sub_router.needs_matched_route(framework_items_db) {
                let path = route_id2path.get_by_left(route_id).unwrap();
                quote! {
                    let matched_route_template = #pavex::request::path::MatchedPathPattern::new(
                        #path
                    );
                }
            } else {
                quote! {}
            };
            let mut fallback_invocation = sub_router.catch_all_pipeline.entrypoint_invocation(
                singleton_bindings,
                request_scoped_bindings,
                &server_state_ident,
            );
            if fallback_codegened_pipeline.needs_connection_info(framework_items_db) {
                needs_connection_info = true;
                fallback_invocation = quote! {
                    {
                        let connection_info = connection_info.expect("Required ConnectionInfo is missing");
                        #fallback_invocation
                    }
                }
            }
            if sub_router
                .catch_all_pipeline
                .needs_allowed_methods(framework_items_db)
            {
                fallback_invocation = quote! {
                    {
                        #allowed_methods_init
                        #fallback_invocation
                    }
                };
            }
            quote! {
                {
                    #matched_route_template
                    match &request_head.method {
                        #sub_router_dispatch_table
                        _ => #fallback_invocation,
                    }
                }
            }
        };
        route_dispatch_table = quote! {
            #route_dispatch_table
            #route_id => #match_arm,
        };
    }

    let root_fallback_invocation = fallback_codegened_pipeline.entrypoint_invocation(
        singleton_bindings,
        request_scoped_bindings,
        &server_state_ident,
    );
    let unmatched_route = if fallback_codegened_pipeline.needs_matched_route(framework_items_db) {
        quote! {
            let matched_route_template = #pavex::request::path::MatchedPathPattern::new("*");
        }
    } else {
        quote! {}
    };
    let allowed_methods = if fallback_codegened_pipeline.needs_allowed_methods(framework_items_db) {
        quote! {
            let allowed_methods: #pavex::router::AllowedMethods = #pavex::router::MethodAllowList::from_iter(vec![]).into();
        }
    } else {
        quote! {}
    };
    let unwrap_connection_info =
        if fallback_codegened_pipeline.needs_connection_info(framework_items_db) {
            needs_connection_info = true;
            quote! {
                let connection_info = connection_info.expect("Required ConnectionInfo is missing");
            }
        } else {
            quote! {}
        };
    let connection_info_ident = if needs_connection_info {
        format_ident!("connection_info")
    } else {
        format_ident!("_connection_info")
    };
    syn::parse2(quote! {
        async fn route_request(
            request: #http::Request<#hyper::body::Incoming>,
            #connection_info_ident: Option<#pavex::connection::ConnectionInfo>,
            #server_state_ident: std::sync::Arc<ServerState>
        ) -> #pavex::response::Response {
            let (request_head, request_body) = request.into_parts();
            #[allow(unused)]
            let request_body = #pavex::request::body::RawIncomingBody::from(request_body);
            let request_head: #pavex::request::RequestHead = request_head.into();
            let matched_route = match server_state.router.at(&request_head.target.path()) {
                Ok(m) => m,
                Err(_) => {
                    #allowed_methods
                    #unmatched_route
                    #unwrap_connection_info
                    return #root_fallback_invocation;
                }
            };
            let route_id = matched_route.value;
            #[allow(unused)]
            let url_params: #pavex::request::path::RawPathParams<'_, '_> = matched_route
                .params
                .into();
            match route_id {
                #route_dispatch_table
                i => unreachable!("Unknown route id: {}", i),
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
                version: Some(version.to_string()),
                ..DependencyDetail::default()
            };
            if needs_rename {
                dependency_details.package = Some(name.to_string());
            }

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
            };
            let dependency = Dependency::Detailed(dependency_details).simplify();

            dependencies.insert(dependency_name.clone(), dependency);
            package_ids2dependency_name
                .insert(package_id.to_owned(), dependency_name.replace("-", "_"));
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
                    Computation::PrebuiltType(i) => {
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
