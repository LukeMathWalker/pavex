//! Converter for methods (both inherent and trait methods).

use std::ops::Deref;
use std::sync::Arc;

use rustdoc_types::{GenericArgs, Item, ItemEnum};

use rustdoc_ext::GlobalItemId;
use rustdoc_ir::{
    Callable, CallableInput, FnHeader, GenericArgument, GenericLifetimeParameter, InherentMethod,
    InherentMethodPath, RustIdentifier, TraitMethod, TraitMethodPath,
};
use rustdoc_processor::CrateCollection;
use rustdoc_processor::indexing::CrateIndexer;
use rustdoc_processor::queries::Crate;

use crate::GenericBindings;
use crate::errors::*;
use crate::resolve_type::{TypeAliasResolution, resolve_type};

/// Convert a method item retrieved from `rustdoc`'s JSON output to Pavex's internal
/// representation for callables (i.e. methods and functions).
///
/// # Input constraints
///
/// - `method_item` belongs to `krate`.
/// - `impl_id` is local to `krate`.
/// - `attached_to` can either point to a trait or a type.
///   It'll always point to the `Self` type if we're working with an inherent method.
///
/// `attached_to`, in the trait case, may not be local to `krate`.
/// E.g. the user is implementing a trait defined in another crate
/// for one of their local types.
pub fn rustdoc_method2callable<I: CrateIndexer>(
    attached_to: rustdoc_types::Id,
    impl_id: rustdoc_types::Id,
    method_item: &Item,
    krate: &Crate,
    krate_collection: &CrateCollection<I>,
    alias_resolution: TypeAliasResolution,
) -> Result<Callable, CallableResolutionError> {
    let impl_item = krate.get_item_by_local_type_id(&impl_id);
    let ItemEnum::Impl(impl_item) = &impl_item.inner else {
        unreachable!("The impl item id doesn't point to an impl item")
    };

    let mut generic_bindings = GenericBindings::default();

    let self_ty = match resolve_type(
        &impl_item.for_,
        &krate.core.package_id,
        krate_collection,
        &generic_bindings,
        alias_resolution,
    ) {
        Ok(t) => t,
        Err(e) => {
            let path_display: String = krate.import_index.items[&attached_to]
                .canonical_path()
                .iter()
                .cloned()
                .chain(std::iter::once(
                    method_item.name.clone().expect("Method without a name"),
                ))
                .collect::<Vec<_>>()
                .join("::");
            return Err(SelfResolutionError {
                path: path_display,
                source: Arc::new(e.into()),
            }
            .into());
        }
    };

    generic_bindings
        .types
        .insert("Self".into(), self_ty.clone());

    // Build path before resolving inputs/outputs so we can use it in error messages.
    enum MethodPath {
        Trait(TraitMethodPath),
        Inherent(InherentMethodPath),
    }
    impl std::fmt::Display for MethodPath {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                MethodPath::Trait(p) => write!(f, "{p}"),
                MethodPath::Inherent(p) => write!(f, "{p}"),
            }
        }
    }
    let callable_path = if let Some(trait_) = &impl_item.trait_ {
        let (trait_global_id, trait_path) = krate_collection
            .get_canonical_path_by_local_type_id(&krate.core.package_id, &trait_.id, None)
            // FIXME: handle the error
            .unwrap();
        let method_name = method_item.name.clone().expect("Method without a name");

        let mut trait_generics = Vec::new();
        if let Some(args) = &trait_.args {
            let GenericArgs::AngleBracketed { args, .. } = args.as_ref() else {
                todo!();
            };
            for arg in args {
                let ga = match arg {
                    rustdoc_types::GenericArg::Lifetime(l) => {
                        GenericArgument::Lifetime(GenericLifetimeParameter::from_name(l))
                    }
                    rustdoc_types::GenericArg::Type(t) => {
                        let Ok(t) = resolve_type(
                            t,
                            &krate.core.package_id,
                            krate_collection,
                            &generic_bindings,
                            alias_resolution,
                        ) else {
                            todo!()
                        };
                        GenericArgument::TypeParameter(t)
                    }
                    rustdoc_types::GenericArg::Const(constant) => {
                        let raw = constant.value.as_ref().unwrap_or(&constant.expr);
                        let value = generic_bindings
                            .consts
                            .get(raw)
                            .cloned()
                            .unwrap_or_else(|| raw.clone());
                        GenericArgument::Const(rustdoc_ir::ConstGenericArgument { value })
                    }
                    rustdoc_types::GenericArg::Infer => {
                        unreachable!()
                    }
                };
                trait_generics.push(ga);
            }
        }

        let n = trait_path.len();
        MethodPath::Trait(TraitMethodPath {
            package_id: trait_global_id.package_id.clone(),
            crate_name: trait_path[0].clone(),
            module_path: trait_path[1..n - 1].to_vec(),
            trait_name: trait_path[n - 1].clone(),
            trait_generics,
            self_type: self_ty.clone(),
            method_name,
            method_generics: vec![],
        })
    } else {
        let canonical_path_segments: Vec<String> = krate.import_index.items[&attached_to]
            .canonical_path()
            .to_vec();
        let method_name = method_item.name.clone().expect("Method without a name");
        let n = canonical_path_segments.len();
        MethodPath::Inherent(InherentMethodPath {
            package_id: krate.core.package_id.clone(),
            crate_name: canonical_path_segments[0].clone(),
            module_path: canonical_path_segments[1..n - 1].to_vec(),
            type_name: canonical_path_segments[n - 1].clone(),
            type_generics: vec![],
            method_name,
            method_generics: vec![],
        })
    };

    let ItemEnum::Function(inner) = &method_item.inner else {
        unreachable!("Expected a function item");
    };

    let mut inputs = Vec::new();
    let mut takes_self_as_ref = false;
    for (parameter_index, (param_name, parameter_type)) in inner.sig.inputs.iter().enumerate() {
        if parameter_index == 0 {
            // The first parameter might be `&self` or `&mut self`.
            // This is important to know for carrying out further analysis doing the line,
            // e.g. undoing lifetime elision.
            if let rustdoc_types::Type::BorrowedRef { type_, .. } = parameter_type
                && let rustdoc_types::Type::Generic(g) = type_.deref()
                && g == "Self"
            {
                takes_self_as_ref = true;
            }
        }

        match resolve_type(
            parameter_type,
            &krate.core.package_id,
            krate_collection,
            &generic_bindings,
            alias_resolution,
        ) {
            Ok(t) => {
                inputs.push(CallableInput {
                    name: RustIdentifier::new(param_name.clone()),
                    type_: t,
                });
            }
            Err(e) => {
                return Err(InputParameterResolutionError {
                    callable_path: callable_path.to_string(),
                    callable_item: method_item.clone(),
                    parameter_type: parameter_type.clone(),
                    parameter_index,
                    source: Arc::new(e.into()),
                }
                .into());
            }
        }
    }

    let output = match &inner.sig.output {
        Some(output_ty) => {
            match resolve_type(
                output_ty,
                &krate.core.package_id,
                krate_collection,
                &generic_bindings,
                alias_resolution,
            ) {
                Ok(t) => Some(t),
                Err(e) => {
                    return Err(OutputTypeResolutionError {
                        callable_path: callable_path.to_string(),
                        callable_item: method_item.clone(),
                        output_type: output_ty.clone(),
                        source: Arc::new(e.into()),
                    }
                    .into());
                }
            }
        }
        None => None,
    };

    let symbol_name = method_item.attrs.iter().find_map(|attr| match attr {
        rustdoc_types::Attribute::NoMangle => method_item.name.clone(),
        rustdoc_types::Attribute::ExportName(name) => Some(name.clone()),
        _ => None,
    });

    let fn_header = FnHeader {
        output,
        inputs,
        is_async: inner.header.is_async,
        abi: inner.header.abi.clone(),
        is_unsafe: inner.header.is_unsafe,
        is_c_variadic: inner.sig.is_c_variadic,
        symbol_name,
    };
    let source_coordinates = Some(GlobalItemId {
        rustdoc_item_id: method_item.id,
        package_id: krate.core.package_id.clone(),
    });
    Ok(match callable_path {
        MethodPath::Trait(path) => Callable::TraitMethod(TraitMethod {
            path,
            header: fn_header,
            source_coordinates,
            takes_self_as_ref,
        }),
        MethodPath::Inherent(path) => Callable::InherentMethod(InherentMethod {
            path,
            header: fn_header,
            source_coordinates,
            takes_self_as_ref,
        }),
    })
}
