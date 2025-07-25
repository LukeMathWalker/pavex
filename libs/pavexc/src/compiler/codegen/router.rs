use std::collections::{BTreeMap, BTreeSet};

use ahash::HashSet;
use bimap::{BiBTreeMap, BiHashMap};
use guppy::PackageId;
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, ImplItemFn, ItemFn};

use crate::{
    compiler::analyses::{
        application_state::ApplicationState,
        components::ComponentId,
        domain::DomainGuard,
        framework_items::FrameworkItemDb,
        processing_pipeline::CodegenedRequestHandlerPipeline,
        router::{PathRouter, Router},
    },
    language::ResolvedType,
    utils::syn_debug_parse2,
};

use super::deps::ServerSdkDeps;

pub(super) fn codegen_router(
    router: &Router,
    sdk_deps: &ServerSdkDeps,
    handler_id2codegened_pipeline: &BTreeMap<ComponentId, CodegenedRequestHandlerPipeline>,
    application_state: &ApplicationState,
    request_scoped_bindings: &BiHashMap<Ident, ResolvedType>,
    package_id2name: &BiHashMap<PackageId, String>,
    framework_items_db: &FrameworkItemDb,
) -> TokenStream {
    let struct_ = router_struct(router, sdk_deps);
    let impl_ = router_impl(
        router,
        sdk_deps,
        handler_id2codegened_pipeline,
        application_state,
        request_scoped_bindings,
        package_id2name,
        framework_items_db,
    );
    quote! {
        #struct_
        #impl_
    }
}

/// Generate the struct definition for the router.
fn router_struct(router: &Router, sdk_deps: &ServerSdkDeps) -> TokenStream {
    let matchit = sdk_deps.matchit_ident();
    match router {
        Router::DomainAgnostic(_) => {
            quote! {
                struct Router {
                    router: #matchit::Router<u32>
                }
            }
        }
        Router::DomainBased(router) => {
            let domain_routers =
                (0..router.domain2path_router.len()).map(|i| format_ident!("domain_{i}"));
            quote! {
                struct Router {
                    domain_router: #matchit::Router<u32>,
                    #(#domain_routers: #matchit::Router<u32>,)*
                }
            }
        }
    }
}

fn router_impl(
    router: &Router,
    sdk_deps: &ServerSdkDeps,
    handler_id2codegened_pipeline: &BTreeMap<ComponentId, CodegenedRequestHandlerPipeline>,
    application_state: &ApplicationState,
    request_scoped_bindings: &BiHashMap<Ident, ResolvedType>,
    package_id2name: &BiHashMap<PackageId, String>,
    framework_items_db: &FrameworkItemDb,
) -> TokenStream {
    let route_method_ident = format_ident!("route");
    match router {
        Router::DomainAgnostic(router) => {
            let (route_id2path, route_id2method_router) =
                route_mappings(router, handler_id2codegened_pipeline);
            let router_init_method_name = format_ident!("router");

            let mut router_init = path_router_init(&route_id2path, sdk_deps);
            router_init.sig.ident = router_init_method_name.clone();

            let mut route_request = path_router(
                &format_ident!("router"),
                &route_id2method_router,
                &route_id2path,
                &handler_id2codegened_pipeline[&router.root_fallback_id],
                application_state,
                request_scoped_bindings,
                framework_items_db,
                package_id2name,
                sdk_deps,
            );
            route_request.vis = syn::Visibility::Public(Default::default());
            route_request.sig.ident = route_method_ident;

            quote! {
                impl Router {
                    /// Create a new router instance.
                    ///
                    /// This method is invoked once, when the server starts.
                    pub fn new() -> Self {
                        Self { router: Self::#router_init_method_name() }
                    }
                    #router_init
                    #route_request
                }
            }
        }
        Router::DomainBased(router) => {
            let mut init_fns = Vec::new();
            let mut route_fns = Vec::new();
            for (i, sub_router) in router.domain2path_router.values().enumerate() {
                let (route_id2path, route_id2method_router) =
                    route_mappings(sub_router, handler_id2codegened_pipeline);
                let router_init_method_name = format_ident!("domain_{i}_router");

                let mut router_init = path_router_init(&route_id2path, sdk_deps);
                router_init.sig.ident = router_init_method_name.clone();

                let mut route_request = path_router(
                    &format_ident!("domain_{i}"),
                    &route_id2method_router,
                    &route_id2path,
                    &handler_id2codegened_pipeline[&sub_router.root_fallback_id],
                    application_state,
                    request_scoped_bindings,
                    framework_items_db,
                    package_id2name,
                    sdk_deps,
                );
                route_request.sig.ident = format_ident!("route_domain_{i}");

                init_fns.push(router_init);
                route_fns.push(route_request);
            }

            let domain_router_init_fn = domain_router_init(&router.domain2path_router, sdk_deps);

            let domain_route_fn = domain_router(
                &router.domain2path_router,
                &handler_id2codegened_pipeline[&router.root_fallback_id],
                application_state,
                request_scoped_bindings,
                framework_items_db,
                package_id2name,
                sdk_deps,
            );

            let router_new = {
                let fields = init_fns.iter().enumerate().map(|(i, init_fn)| {
                    let field_name = format_ident!("domain_{i}");
                    let init_name = &init_fn.sig.ident;
                    quote! {
                        #field_name: Self::#init_name()
                    }
                });
                quote! {
                    /// Create a new router instance.
                    ///
                    /// This method is invoked once, when the server starts.
                    pub fn new() -> Self {
                        Self {
                            domain_router: Self::domain_router(),
                            #(#fields,)*
                        }
                    }
                }
            };

            quote! {
                impl Router {
                    #router_new
                    #domain_router_init_fn
                    #(#init_fns)*
                    #domain_route_fn
                    #(#route_fns)*
                }
            }
        }
    }
}

/// Compute the route mappings required to generate the underlying `matchit` router as well as
/// the routing logic.
fn route_mappings(
    router: &PathRouter,
    handler_id2codegened_pipeline: &BTreeMap<ComponentId, CodegenedRequestHandlerPipeline>,
) -> (BiBTreeMap<u32, String>, BTreeMap<u32, CodegenMethodRouter>) {
    let mut path2codegen_router_entry = IndexMap::new();
    for (path, method_router) in router.path2method_router.iter() {
        let mut methods_and_pipelines = Vec::with_capacity(method_router.handler_id2methods.len());
        for (handler_id, methods) in &method_router.handler_id2methods {
            let pipeline = &handler_id2codegened_pipeline[handler_id];
            methods_and_pipelines.push((methods.clone(), pipeline.clone()));
        }
        let catch_all_pipeline = handler_id2codegened_pipeline[&method_router.fallback_id].clone();
        path2codegen_router_entry.insert(
            path.to_owned(),
            CodegenMethodRouter {
                methods_and_pipelines,
                catch_all_pipeline,
            },
        );
    }
    let mut route_id2path = BiBTreeMap::new();
    let mut route_id2router_entry = BTreeMap::new();
    for (route_id, (path, router_entry)) in path2codegen_router_entry.iter().enumerate() {
        route_id2path.insert(route_id as u32, path.to_owned());
        route_id2router_entry.insert(route_id as u32, router_entry.to_owned());
    }

    (route_id2path, route_id2router_entry)
}

fn path_router_init(route_id2path: &BiBTreeMap<u32, String>, sdk_deps: &ServerSdkDeps) -> ItemFn {
    let matchit = sdk_deps.matchit_ident();
    let router = format_ident!("router");
    let inserts = route_id2path.iter().map(|(route_id, path)| {
        quote! {
            #router.insert(#path, #route_id).unwrap();
        }
    });
    let mut_ = (!route_id2path.is_empty()).then(|| quote! { mut });
    syn::parse2(quote! {
        fn router() -> #matchit::Router<u32> {
            let #mut_ #router = #matchit::Router::new();
            // Pavex has validated at compile-time that all route paths are valid
            // and that there are no conflicts, therefore we can safely unwrap
            // every `insert`.
            #(#inserts)*
            #router
        }
    })
    .unwrap()
}

fn domain_router_init(
    domain2path_router: &BTreeMap<DomainGuard, PathRouter>,
    sdk_deps: &ServerSdkDeps,
) -> ItemFn {
    let matchit = sdk_deps.matchit_ident();
    let router = format_ident!("router");
    let inserts = domain2path_router.keys().enumerate().map(|(i, guard)| {
        let pattern = guard.matchit_pattern();
        let i = i as u32;
        quote! {
            #router.insert(#pattern, #i).unwrap();
        }
    });
    syn::parse2(quote! {
        fn domain_router() -> #matchit::Router<u32> {
            let mut #router = #matchit::Router::new();
            // Pavex has validated at compile-time that all domain patterns are valid
            // and that there are no conflicts, therefore we can safely unwrap
            // every `insert`.
            #(#inserts)*
            #router
        }
    })
    .unwrap()
}

fn domain_router(
    domain2path_router: &BTreeMap<DomainGuard, PathRouter>,
    fallback_codegened_pipeline: &CodegenedRequestHandlerPipeline,
    application_state: &ApplicationState,
    request_scoped_bindings: &BiHashMap<Ident, ResolvedType>,
    framework_items_db: &FrameworkItemDb,
    package_id2name: &BiHashMap<PackageId, String>,
    sdk_deps: &ServerSdkDeps,
) -> ItemFn {
    let http = sdk_deps.http_ident();
    let hyper = sdk_deps.hyper_ident();
    let pavex = sdk_deps.pavex_ident();
    let request = format_ident!("request");
    let state = format_ident!("state");
    let connection_info = framework_items_db.get_binding(FrameworkItemDb::connection_info_id());

    let domain_dispatch_arms = domain2path_router.iter().enumerate().map(|(i, _)| {
        let domain_router_method_name = format_ident!("route_domain_{i}");
        let i = i as u32;
        quote! {
            #i => self.#domain_router_method_name(#request, #connection_info, #state).await,
        }
    });

    let root_fallback_invocation = routing_failure_fallback_block(
        fallback_codegened_pipeline,
        application_state,
        request_scoped_bindings,
        framework_items_db,
        &state,
        package_id2name,
        sdk_deps,
        false,
    );
    let request_head_id = FrameworkItemDb::request_head_id();
    let request_head_ident = framework_items_db.get_binding(request_head_id);
    let request_head_ty = framework_items_db
        .get_type(request_head_id)
        .syn_type(package_id2name);
    let request_body_id = FrameworkItemDb::raw_incoming_body_id();
    let request_body_ident = framework_items_db.get_binding(request_body_id);
    let request_body_ty = framework_items_db
        .get_type(request_body_id)
        .syn_type(package_id2name);
    let generated = quote! {
        pub async fn route(
            &self,
            #request: #http::Request<#hyper::body::Incoming>,
            #connection_info: Option<#pavex::connection::ConnectionInfo>,
            #state: &ApplicationState
        ) -> #pavex::Response {
            let host: Option<String> = #request
                .headers()
                .get(#pavex::http::header::HOST)
                .map(|h| #pavex::http::uri::Authority::try_from(h.as_bytes()).ok())
                .flatten()
                .map(|a| a.host()
                    // Normalize the host by removing the trailing dot, if it exists.
                    .trim_end_matches('.')
                    // Replace dots with slashes, since that's the separator that `matchit` understands.
                    .replace('.', "/")
                    // Reverse the string to maximise shared prefixes in the underlying `matchit` router.
                    .chars().rev().collect()
                );

            if let Some(host) = host {
                if let Ok(m) = self.domain_router.at(host.as_str()) {
                    return match m.value {
                        #(#domain_dispatch_arms)*
                        i => unreachable!("Unknown domain id: {}", i),
                    };
                }
            }

            // No domain matched, or the request did not contain a valid `Host` header.
            // Time to invoke the fallback route.
            let (request_head, request_body) = #request.into_parts();
            #[allow(unused)]
            let #request_body_ident = #request_body_ty::from(request_body);
            let #request_head_ident: #request_head_ty = request_head.into();
            #root_fallback_invocation
        }
    };
    syn::parse2(generated).unwrap()
}

fn path_router(
    router_field_name: &Ident,
    route_id2method_router: &BTreeMap<u32, CodegenMethodRouter>,
    route_id2path: &BiBTreeMap<u32, String>,
    fallback_codegened_pipeline: &CodegenedRequestHandlerPipeline,
    application_state: &ApplicationState,
    request_scoped_bindings: &BiHashMap<Ident, ResolvedType>,
    framework_item_db: &FrameworkItemDb,
    package_id2name: &BiHashMap<PackageId, String>,
    sdk_deps: &ServerSdkDeps,
) -> ImplItemFn {
    static WELL_KNOWN_METHODS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
        HashSet::from_iter([
            "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "CONNECT", "TRACE",
        ])
    });

    let pavex = sdk_deps.pavex_ident();
    let http = sdk_deps.http_ident();
    let hyper = sdk_deps.hyper_ident();
    let mut route_match_arms = Vec::new();
    let server_state_ident = format_ident!("state");
    let connection_info_ident =
        framework_item_db.get_binding(FrameworkItemDb::connection_info_id());
    let request_head_id = FrameworkItemDb::request_head_id();
    let request_head_ident = framework_item_db.get_binding(request_head_id);
    let request_head_ty = framework_item_db
        .get_type(request_head_id)
        .syn_type(package_id2name);

    let needs_framework_item = |id| {
        route_id2method_router.values().any(|r| {
            r.methods_and_pipelines
                .iter()
                .any(|(_, p)| p.needs_framework_item(framework_item_db, id))
                || r.catch_all_pipeline
                    .needs_framework_item(framework_item_db, id)
        }) || fallback_codegened_pipeline.needs_framework_item(framework_item_db, id)
    };

    let needs_request_body = needs_framework_item(FrameworkItemDb::raw_incoming_body_id());
    let needs_connection_info = needs_framework_item(FrameworkItemDb::connection_info_id());
    let needs_url_params = needs_framework_item(FrameworkItemDb::url_params_id());

    for (route_id, sub_router) in route_id2method_router {
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
                        let expect_msg = format!("{m} is not a valid (custom) HTTP method");
                        quote! {
                            #pavex::http::Method::try_from(#m).expect(#expect_msg)
                        }
                    }
                });
            let allowed_methods_id = FrameworkItemDb::allowed_methods_id();
            let allowed_methods_ident = framework_item_db.get_binding(allowed_methods_id);
            let allowed_methods_ty = framework_item_db
                .get_type(allowed_methods_id)
                .syn_type(package_id2name);
            quote! {
                let #allowed_methods_ident: #allowed_methods_ty = #pavex::router::MethodAllowList::from_iter(
                    [#(#allowed_methods),*]
                ).into();
            }
        };

        let connection_info_init = quote! {
            let #connection_info_ident = #connection_info_ident.expect("Required `ConnectionInfo` is missing");
        };

        let matched_route_init = {
            let path = route_id2path.get_by_left(route_id).unwrap();
            let id = FrameworkItemDb::matched_route_template_id();
            let ident = framework_item_db.get_binding(id);
            let ty_ = framework_item_db.get_type(id).syn_type(package_id2name);
            quote! {
                let #ident = #ty_::new(
                    #path
                );
            }
        };

        let codegen_invocation = |pipeline: &CodegenedRequestHandlerPipeline| {
            let invocation = pipeline.entrypoint_invocation(
                application_state,
                request_scoped_bindings,
                &server_state_ident,
            );
            let mut framework_primitives = Vec::new();
            if pipeline.needs_allowed_methods(framework_item_db) {
                framework_primitives.push(allowed_methods_init.clone());
            }
            if pipeline.needs_connection_info(framework_item_db) {
                framework_primitives.push(connection_info_init.clone());
            }
            if pipeline.needs_matched_route(framework_item_db) {
                framework_primitives.push(matched_route_init.clone());
            }
            quote! {
                {
                    #(#framework_primitives)*
                    #invocation
                }
            }
        };

        let match_arm = if sub_router.methods_and_pipelines.is_empty() {
            // We just have the catch-all handler, we can skip the `match`.
            let request_pipeline = &sub_router.catch_all_pipeline;
            codegen_invocation(request_pipeline)
        } else {
            let mut sub_router_dispatch_table = quote! {};

            for (methods, request_pipeline) in &sub_router.methods_and_pipelines {
                let invocation = codegen_invocation(request_pipeline);

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
            let fallback_invocation = codegen_invocation(&sub_router.catch_all_pipeline);
            quote! {
                {
                    match &#request_head_ident.method {
                        #sub_router_dispatch_table
                        _ => #fallback_invocation,
                    }
                }
            }
        };
        route_match_arms.push(quote! {
            #route_id => #match_arm,
        });
    }

    let root_fallback_invocation = routing_failure_fallback_block(
        fallback_codegened_pipeline,
        application_state,
        request_scoped_bindings,
        framework_item_db,
        &server_state_ident,
        package_id2name,
        sdk_deps,
        true,
    );
    let connection_info_ident = if needs_connection_info {
        connection_info_ident.to_owned()
    } else {
        format_ident!("_{}", connection_info_ident)
    };
    let request_transformation = if needs_request_body {
        let id = FrameworkItemDb::raw_incoming_body_id();
        let ident = framework_item_db.get_binding(id);
        let ty_ = framework_item_db.get_type(id).syn_type(package_id2name);
        quote! {
            let (request_head, request_body) = request.into_parts();
            let #request_head_ident: #request_head_ty = request_head.into();
            let #ident = #ty_::from(request_body);
        }
    } else {
        quote! {
            let (request_head, _) = request.into_parts();
            let #request_head_ident: #request_head_ty = request_head.into();
        }
    };
    let matched_route_ident = format_ident!("matched_route");
    let url_params = needs_url_params.then(|| {
        let ident = framework_item_db.get_binding(FrameworkItemDb::url_params_id());
        quote! {
            let #ident: #pavex::request::path::RawPathParams<'_, '_> = #matched_route_ident
                .params
                .into();
        }
    });
    let code = quote! {
        async fn route(
            &self,
            request: #http::Request<#hyper::body::Incoming>,
            #connection_info_ident: Option<#pavex::connection::ConnectionInfo>,
            #[allow(unused)]
            #server_state_ident: &ApplicationState
        ) -> #pavex::Response {
            #request_transformation
            let Ok(#matched_route_ident) = self.#router_field_name.at(&#request_head_ident.target.path()) else {
                #root_fallback_invocation
            };
            #url_params
            match #matched_route_ident.value {
                #(#route_match_arms)*
                i => unreachable!("Unknown route id: {}", i),
            }
        }
    };
    syn_debug_parse2(code)
}

fn routing_failure_fallback_block(
    fallback_codegened_pipeline: &CodegenedRequestHandlerPipeline,
    application_state: &ApplicationState,
    request_scoped_bindings: &BiHashMap<Ident, ResolvedType>,
    framework_items_db: &FrameworkItemDb,
    server_state_ident: &Ident,
    package_id2name: &BiHashMap<PackageId, String>,
    sdk_deps: &ServerSdkDeps,
    return_: bool,
) -> TokenStream {
    let pavex = sdk_deps.pavex_ident();
    let root_fallback_invocation = fallback_codegened_pipeline.entrypoint_invocation(
        application_state,
        request_scoped_bindings,
        server_state_ident,
    );
    let unmatched_route = fallback_codegened_pipeline
        .needs_matched_route(framework_items_db)
        .then(|| {
            let id = FrameworkItemDb::matched_route_template_id();
            let ident = framework_items_db.get_binding(id);
            let ty_ = framework_items_db.get_type(id).syn_type(package_id2name);
            quote! {
                let #ident = #ty_::new("*");
            }
        });
    let allowed_methods = fallback_codegened_pipeline
        .needs_allowed_methods(framework_items_db)
        .then(|| {
            let id = FrameworkItemDb::allowed_methods_id();
            let ident = framework_items_db.get_binding(id);
            let ty_ = framework_items_db.get_type(id).syn_type(package_id2name);
            quote! {
                let #ident: #ty_ = #pavex::router::MethodAllowList::from_iter(vec![]).into();
            }
        });
    let url_params = fallback_codegened_pipeline
        .needs_url_params(framework_items_db)
        .then(|| {
            let id = FrameworkItemDb::url_params_id();
            let ident = framework_items_db.get_binding(id);
            quote! {
                let #ident = #pavex::request::path::RawPathParams::default();
            }
        });
    let unwrap_connection_info = fallback_codegened_pipeline
        .needs_connection_info(framework_items_db)
        .then(|| {
            let connection_info_ident = framework_items_db.get_binding(FrameworkItemDb::connection_info_id());
            quote! {
                let #connection_info_ident = #connection_info_ident.expect("Required ConnectionInfo is missing");
            }
        });
    let invocation = if return_ {
        quote! {
            return #root_fallback_invocation;
        }
    } else {
        quote! {
            #root_fallback_invocation
        }
    };
    quote! {
        #url_params
        #allowed_methods
        #unmatched_route
        #unwrap_connection_info
        #invocation
    }
}

#[derive(Debug, Clone)]
/// A router that dispatches requests based on their HTTP method.
pub(super) struct CodegenMethodRouter {
    pub(super) methods_and_pipelines: Vec<(BTreeSet<String>, CodegenedRequestHandlerPipeline)>,
    pub(super) catch_all_pipeline: CodegenedRequestHandlerPipeline,
}
