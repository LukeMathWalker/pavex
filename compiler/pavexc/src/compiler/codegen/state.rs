use std::collections::BTreeMap;

use bimap::BiHashMap;
use guppy::PackageId;
use indexmap::IndexMap;
use quote::{format_ident, quote};
use syn::{ItemEnum, ItemFn, ItemStruct};

use crate::compiler::{
    analyses::{
        application_config::ApplicationConfig, application_state::ApplicationState,
        computations::ComputationDb,
    },
    codegen_utils::VariableNameGenerator,
};

use super::{ApplicationStateCallGraph, ComponentDb, ResolvedType, deps::ServerSdkDeps};

pub(super) fn define_application_state(
    application_state: &ApplicationState,
    package_id2name: &BiHashMap<PackageId, String>,
) -> ItemStruct {
    let bindings = application_state
        .bindings()
        .iter()
        .map(|(field_name, type_)| {
            let field_type = type_.syn_type(package_id2name);
            (field_name, field_type)
        })
        .collect::<BTreeMap<_, _>>();

    let fields = bindings.iter().map(|(field_name, type_)| {
        quote! { pub #field_name: #type_ }
    });
    syn::parse2(quote! {
        pub struct ApplicationState {
            #(#fields),*
        }
    })
    .unwrap()
}

pub(super) fn define_application_config(
    config: &ApplicationConfig,
    package_id2name: &BiHashMap<PackageId, String>,
    sdk_deps: &ServerSdkDeps,
) -> ItemStruct {
    let bindings = config
        .bindings()
        .iter()
        .map(|(field_name, type_)| {
            let field_type = type_.syn_type(package_id2name);
            (field_name, field_type)
        })
        .collect::<BTreeMap<_, _>>();

    let fields = bindings.iter().map(|(field_name, type_)| {
        let attr = config.should_default(field_name).then(|| {
            quote! {
                #[serde(default)]
            }
        });
        quote! {
            #attr
            pub #field_name: #type_
        }
    });
    let serde = sdk_deps.serde_ident();
    syn::parse2(quote! {
        #[derive(Debug, Clone, #serde::Deserialize)]
        pub struct ApplicationConfig {
            #(#fields),*
        }
    })
    .unwrap()
}

pub(super) fn define_application_state_error(
    error_types: &IndexMap<String, ResolvedType>,
    package_id2name: &BiHashMap<PackageId, String>,
    sdk_deps: &ServerSdkDeps,
) -> Result<ItemEnum, anyhow::Error> {
    let thiserror = sdk_deps.thiserror_ident();
    let singleton_fields = error_types.iter().map(|(variant_name, type_)| {
        let variant_type = type_.syn_type(package_id2name);
        let variant_name = format_ident!("{}", variant_name);
        quote! {
            #[error(transparent)]
            #variant_name(#variant_type)
        }
    });
    Ok(syn::parse2(quote! {
        #[derive(Debug, #thiserror::Error)]
        pub enum ApplicationStateError {
            #(#singleton_fields),*
        }
    })?)
}

/// Get the `ApplicationState::new` function.
///
/// To minimise friction for users:
///
/// - `new` always takes an `ApplicationConfig` instance, even if unused, as its first input parameter.
/// - `new` always returns a `Result<ApplicationState, ApplicationStateError>`, even if infallible.
/// - `new` is always `async`, even if unnecessary.
pub(super) fn get_application_state_new(
    application_state_private_new: &ItemFn,
    application_state_call_graph: &ApplicationStateCallGraph,
    application_config: &ApplicationConfig,
    package_id2name: &BiHashMap<PackageId, String>,
) -> Result<ItemFn, anyhow::Error> {
    let input_types = &application_state_call_graph
        .call_graph
        .required_input_types();
    let is_async = application_state_private_new.sig.asyncness.is_some();
    let mut config_ident = format_ident!("app_config");
    let mut variable_name_generator = VariableNameGenerator::new();
    let mut reduced_input_parameters = Vec::new();
    let mut used_config = false;
    let invocation_parameters: Vec<_> = input_types
        .iter()
        .map(|type_| {
            let mut is_shared_reference = false;
            let inner_type = match type_ {
                ResolvedType::Reference(r) => {
                    if !r.lifetime.is_static() {
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
                ResolvedType::Generic(_) => {
                    unreachable!("Generic types should have been resolved by now")
                }
            };
            if let Some(field_name) = application_config.bindings().get_by_right(inner_type) {
                used_config = true;
                if is_shared_reference {
                    quote! {
                        &#config_ident.#field_name
                    }
                } else {
                    quote! {
                        #config_ident.#field_name
                    }
                }
            } else {
                let variable_name = variable_name_generator.generate();
                let type_ = inner_type.syn_type(package_id2name);
                reduced_input_parameters.push(quote! { #variable_name: #type_ });
                quote! {
                    #variable_name
                }
            }
        })
        .collect();
    if !used_config {
        config_ident = format_ident!("_app_config");
    }
    let inner_fn_name = &application_state_private_new.sig.ident;
    let mut invocation = quote! { Self::#inner_fn_name(#(#invocation_parameters),*) };
    if is_async {
        invocation = quote! { #invocation.await };
    }
    let async_keyword = is_async.then(|| quote! { async });
    if application_state_call_graph.error_variants.is_empty() {
        invocation = quote! {
            Ok(#invocation)
        };
    }
    let fn_ = syn::parse2(quote! {
        pub #async_keyword fn new(
            #config_ident: crate::ApplicationConfig,
            #(#reduced_input_parameters),*
        ) -> Result<crate::ApplicationState, crate::ApplicationStateError> {
            #invocation
        }
    })?;
    Ok(fn_)
}

#[tracing::instrument("Codegen application state initialization function", skip_all)]
pub(super) fn get_application_state_private_new(
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
    function.sig.ident = format_ident!("_new");
    function.vis = syn::Visibility::Inherited;
    let output_type = if application_state_call_graph.error_variants.is_empty() {
        quote! { crate::ApplicationState }
    } else {
        quote! { Result<crate::ApplicationState, crate::ApplicationStateError> }
    };
    function.sig.output =
        syn::ReturnType::Type(Default::default(), Box::new(syn::parse2(output_type)?));
    Ok(function)
}
