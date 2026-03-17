//! Resolution of framework types and callables from rustdoc.

use std::ops::Deref;

use guppy::PackageId;
use rustdoc_types::ItemEnum;

use crate::language::{Callable, GenericArgument, PathType, Type};
use crate::rustdoc::{CannotGetCrateData, CrateCollection};
use rustdoc_ext::GlobalItemId;
use rustdoc_ir::{CallableInput, FnHeader, RustIdentifier, TraitMethod, TraitMethodPath};
use rustdoc_processor::queries::Crate;
use rustdoc_resolver::{GenericBindings, TypeAliasResolution, resolve_type};

use super::app::PAVEX_VERSION;

// Re-export types from `rustdoc_resolver` so that downstream code
// within pavexc can keep importing them from this module.
pub use rustdoc_resolver::{
    InputParameterResolutionError, OutputTypeResolutionError, SelfResolutionError,
    TypeResolutionError, UnsupportedConstGeneric,
};

#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum CallableResolutionError {
    #[error(transparent)]
    SelfResolutionError(#[from] SelfResolutionError),
    #[error(transparent)]
    InputParameterResolutionError(#[from] InputParameterResolutionError),
    #[error(transparent)]
    OutputTypeResolutionError(#[from] OutputTypeResolutionError),
    #[error(transparent)]
    CannotGetCrateData(#[from] CannotGetCrateData),
}

impl From<rustdoc_resolver::CallableResolutionError> for CallableResolutionError {
    fn from(e: rustdoc_resolver::CallableResolutionError) -> Self {
        match e {
            rustdoc_resolver::CallableResolutionError::SelfResolutionError(e) => {
                CallableResolutionError::SelfResolutionError(e)
            }
            rustdoc_resolver::CallableResolutionError::InputParameterResolutionError(e) => {
                CallableResolutionError::InputParameterResolutionError(e)
            }
            rustdoc_resolver::CallableResolutionError::OutputTypeResolutionError(e) => {
                CallableResolutionError::OutputTypeResolutionError(e)
            }
        }
    }
}

/// Get the `PackageId` for the `pavex` crate.
fn pavex_package_id(krate_collection: &CrateCollection) -> PackageId {
    crate::language::krate2package_id("pavex", PAVEX_VERSION, krate_collection.package_graph())
        .expect("Failed to find pavex in the package graph")
}

/// Get the `Crate` for the `pavex` crate.
fn pavex_crate(krate_collection: &CrateCollection) -> &Crate {
    let package_id = pavex_package_id(krate_collection);
    krate_collection
        .get_or_compute(&package_id)
        .expect("Failed to compute pavex crate docs")
}

/// Strip generic arguments from a Rust path string.
///
/// E.g. `"pavex::request::path::RawPathParams::<'server, 'request>"` →
/// `["pavex", "request", "path", "RawPathParams"]`
fn strip_generics_from_path(raw_path: &str) -> Vec<String> {
    // Remove everything from the first '<' onwards, then split on '::'
    let base = if let Some(idx) = raw_path.find('<') {
        // Also strip a trailing `::` before `<` if present (turbofish syntax)
        let before = &raw_path[..idx];
        before.trim_end_matches("::")
    } else {
        raw_path
    };
    base.split("::").map(|s| s.to_owned()).collect()
}

/// Resolve a type by looking it up directly in rustdoc.
///
/// The first segment must be a crate name that is either a toolchain crate or
/// a direct dependency of `pavex`.
///
/// Generic arguments in the path (e.g. `Foo::<'a, T>`) are stripped for the
/// rustdoc lookup; the resolved type's generics come from the type definition.
///
/// The path must resolve to a struct, enum, or trait.
///
/// # Panics
///
/// Panics if the resolved item is not a struct, enum, or trait.
pub(crate) fn resolve_type_path(raw_path: &str, krate_collection: &CrateCollection) -> Type {
    // Strip generic arguments from the path for rustdoc lookup.
    // E.g. "pavex::request::path::RawPathParams::<'server, 'request>"
    // becomes ["pavex", "request", "path", "RawPathParams"]
    let segments: Vec<String> = strip_generics_from_path(raw_path);
    let crate_name = &segments[0];

    // Get the crate for the first segment
    let package_id = if crate_name == "pavex" {
        pavex_package_id(krate_collection)
    } else {
        // For toolchain crates or other dependencies, resolve via the package graph
        // using pavex as the anchor (since these are direct dependencies of pavex)
        let pavex_pid = pavex_package_id(krate_collection);
        crate::language::dependency_name2package_id(
            crate_name,
            &pavex_pid,
            krate_collection.package_graph(),
        )
        .unwrap_or_else(|_| panic!("Failed to find crate `{crate_name}` in the package graph"))
    };

    let krate = krate_collection
        .get_or_compute(&package_id)
        .expect("Failed to compute crate docs");

    let global_id = krate
        .get_item_id_by_path(&segments, krate_collection)
        .expect("Failed to get item by path")
        .unwrap_or_else(|e| panic!("Unknown path: {e:?}"));

    let (canonical_global_id, base_type) = krate_collection
        .get_canonical_path_by_local_type_id(
            &global_id.package_id,
            &global_id.rustdoc_item_id,
            None,
        )
        .expect("Failed to get canonical path");

    let item = krate_collection.get_item_by_global_type_id(&global_id);
    let generic_defs = match &item.inner {
        ItemEnum::Struct(s) => &s.generics.params,
        ItemEnum::Enum(e) => &e.generics.params,
        ItemEnum::Trait(t) => &t.generics.params,
        other => {
            panic!(
                "Expected `{raw_path}` to resolve to a struct, enum, or trait, \
                 but it resolved to a {other:?}"
            );
        }
    };

    let mut generic_arguments = vec![];
    for generic_def in generic_defs {
        let arg = match &generic_def.kind {
            rustdoc_types::GenericParamDefKind::Lifetime { .. } => GenericArgument::Lifetime(
                rustdoc_ir::GenericLifetimeParameter::from_name(&generic_def.name),
            ),
            rustdoc_types::GenericParamDefKind::Type { default, .. } => {
                if let Some(default) = default {
                    let default = resolve_type(
                        default,
                        &global_id.package_id,
                        krate_collection,
                        &GenericBindings::default(),
                        TypeAliasResolution::ResolveThrough,
                    )
                    .expect("Failed to resolve default generic type");
                    GenericArgument::TypeParameter(default)
                } else {
                    GenericArgument::TypeParameter(Type::Generic(rustdoc_ir::Generic {
                        name: generic_def.name.clone(),
                    }))
                }
            }
            rustdoc_types::GenericParamDefKind::Const { default, .. } => {
                if let Some(default) = default {
                    GenericArgument::Const(rustdoc_ir::ConstGenericArgument {
                        value: default.clone(),
                    })
                } else {
                    GenericArgument::Const(rustdoc_ir::ConstGenericArgument {
                        value: generic_def.name.clone(),
                    })
                }
            }
        };
        generic_arguments.push(arg);
    }

    PathType {
        package_id: canonical_global_id.package_id().to_owned(),
        rustdoc_id: Some(canonical_global_id.rustdoc_item_id),
        base_type: base_type.to_vec(),
        generic_arguments,
    }
    .into()
}

/// Resolve a free function from pavex's rustdoc by its path segments.
///
/// E.g. `resolve_framework_free_function(&["pavex", "Error", "to_response"], krate_collection)`.
pub(crate) fn resolve_framework_free_function(
    path_segments: &[&str],
    krate_collection: &CrateCollection,
) -> Callable {
    let krate = pavex_crate(krate_collection);
    let segments: Vec<String> = path_segments.iter().map(|s| s.to_string()).collect();

    let global_id = krate
        .get_item_id_by_path(&segments, krate_collection)
        .expect("Failed to look up free function path")
        .unwrap_or_else(|e| panic!("Unknown free function path {}: {e:?}", segments.join("::")));

    let item = krate.get_item_by_local_type_id(&global_id.rustdoc_item_id);
    let free_fn = rustdoc_resolver::resolve_free_function(
        &item,
        krate,
        krate_collection,
        TypeAliasResolution::ResolveThrough,
    )
    .expect("Failed to resolve free function");
    Callable::FreeFunction(free_fn)
}

/// Resolve an inherent method from pavex's rustdoc.
///
/// `type_path` is the path to the type (e.g. `["pavex", "middleware", "Next"]`).
/// `method_name` is the method name (e.g. `"new"`).
pub(crate) fn resolve_framework_inherent_method(
    type_path: &[&str],
    method_name: &str,
    krate_collection: &CrateCollection,
) -> Callable {
    let krate = pavex_crate(krate_collection);
    let segments: Vec<String> = type_path.iter().map(|s| s.to_string()).collect();

    let type_global_id = krate
        .get_item_id_by_path(&segments, krate_collection)
        .expect("Failed to look up type path")
        .unwrap_or_else(|e| panic!("Unknown type path {}: {e:?}", segments.join("::")));

    let type_item = krate.get_item_by_local_type_id(&type_global_id.rustdoc_item_id);

    // Find the method within the type's impl blocks
    let impls = match &type_item.inner {
        ItemEnum::Struct(s) => &s.impls,
        ItemEnum::Enum(e) => &e.impls,
        _ => panic!("Expected a struct or enum for inherent method lookup"),
    };

    for impl_id in impls {
        let impl_item = krate.get_item_by_local_type_id(impl_id);
        let ItemEnum::Impl(impl_block) = &impl_item.inner else {
            continue;
        };
        // Only look at inherent impls (no trait)
        if impl_block.trait_.is_some() {
            continue;
        }
        for item_id in &impl_block.items {
            let item = krate.get_item_by_local_type_id(item_id);
            if item.name.as_deref() == Some(method_name)
                && matches!(item.inner, ItemEnum::Function(_))
            {
                let callable = rustdoc_resolver::rustdoc_method2callable(
                    type_global_id.rustdoc_item_id,
                    *impl_id,
                    &item,
                    krate,
                    krate_collection,
                    TypeAliasResolution::ResolveThrough,
                )
                .expect("Failed to resolve inherent method");
                return callable;
            }
        }
    }

    panic!(
        "Could not find inherent method `{method_name}` on `{}`",
        segments.join("::")
    );
}

/// Resolve a trait method callable for a specific `Self` type.
///
/// This is used for the `IntoResponse::into_response` case, where we need to
/// resolve the trait method with `Self` bound to a concrete output type.
///
/// `trait_path` is the path to the trait (e.g. `["pavex", "IntoResponse"]`).
/// `method_name` is the method name (e.g. `"into_response"`).
/// `self_type` is the concrete type to bind to `Self`.
pub(crate) fn resolve_framework_trait_method(
    trait_path: &[&str],
    method_name: &str,
    self_type: Type,
    krate_collection: &CrateCollection,
) -> Result<Callable, anyhow::Error> {
    let krate = pavex_crate(krate_collection);
    let segments: Vec<String> = trait_path.iter().map(|s| s.to_string()).collect();

    let trait_global_id = krate
        .get_item_id_by_path(&segments, krate_collection)?
        .map_err(|e| anyhow::anyhow!("Unknown trait path {}: {e:?}", segments.join("::")))?;

    let trait_item = krate.get_item_by_local_type_id(&trait_global_id.rustdoc_item_id);
    let ItemEnum::Trait(trait_def) = &trait_item.inner else {
        anyhow::bail!("Expected a trait item for {}", segments.join("::"));
    };

    // Find the method item within the trait's items
    let (method_item_id, method_item) = trait_def
        .items
        .iter()
        .find_map(|item_id| {
            let item = krate.get_item_by_local_type_id(item_id);
            if item.name.as_deref() == Some(method_name)
                && matches!(item.inner, ItemEnum::Function(_))
            {
                Some((*item_id, item))
            } else {
                None
            }
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Could not find method `{method_name}` in trait `{}`",
                segments.join("::")
            )
        })?;

    let ItemEnum::Function(fn_item) = &method_item.inner else {
        unreachable!()
    };

    let mut generic_bindings = GenericBindings::default();
    generic_bindings
        .types
        .insert("Self".to_string(), self_type.clone());

    // Resolve inputs
    let mut inputs = Vec::new();
    let mut takes_self_as_ref = false;
    for (parameter_index, (param_name, parameter_type)) in fn_item.sig.inputs.iter().enumerate() {
        if parameter_index == 0
            && let rustdoc_types::Type::BorrowedRef { type_, .. } = parameter_type
            && let rustdoc_types::Type::Generic(g) = type_.deref()
            && g == "Self"
        {
            takes_self_as_ref = true;
        }
        let resolved = resolve_type(
            parameter_type,
            &krate.core.package_id,
            krate_collection,
            &generic_bindings,
            TypeAliasResolution::ResolveThrough,
        )
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to resolve parameter {parameter_index} of trait method `{method_name}`: {e}"
            )
        })?;
        inputs.push(CallableInput {
            name: RustIdentifier::new(param_name.clone()),
            type_: resolved,
        });
    }

    // Resolve output
    let output = fn_item
        .sig
        .output
        .as_ref()
        .map(|output_ty| {
            resolve_type(
                output_ty,
                &krate.core.package_id,
                krate_collection,
                &generic_bindings,
                TypeAliasResolution::ResolveThrough,
            )
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to resolve output type of trait method `{method_name}`: {e}"
                )
            })
        })
        .transpose()?;

    let symbol_name = method_item.attrs.iter().find_map(|attr| match attr {
        rustdoc_types::Attribute::NoMangle => method_item.name.clone(),
        rustdoc_types::Attribute::ExportName(name) => Some(name.clone()),
        _ => None,
    });

    // Build canonical path for the trait
    let (canonical_trait_id, canonical_trait_path) = krate_collection
        .get_canonical_path_by_local_type_id(
            &trait_global_id.package_id,
            &trait_global_id.rustdoc_item_id,
            None,
        )?;

    let n = canonical_trait_path.len();
    let path = TraitMethodPath {
        package_id: canonical_trait_id.package_id().to_owned(),
        crate_name: canonical_trait_path[0].clone(),
        module_path: canonical_trait_path[1..n - 1].to_vec(),
        trait_name: canonical_trait_path[n - 1].clone(),
        trait_generics: vec![],
        self_type,
        method_name: method_name.to_owned(),
        method_generics: vec![],
    };

    Ok(Callable::TraitMethod(TraitMethod {
        path,
        header: FnHeader {
            output,
            inputs,
            is_async: fn_item.header.is_async,
            abi: fn_item.header.abi.clone(),
            is_unsafe: fn_item.header.is_unsafe,
            is_c_variadic: fn_item.sig.is_c_variadic,
            symbol_name,
        },
        source_coordinates: Some(GlobalItemId {
            rustdoc_item_id: method_item_id,
            package_id: krate.core.package_id.clone(),
        }),
        takes_self_as_ref,
    }))
}
