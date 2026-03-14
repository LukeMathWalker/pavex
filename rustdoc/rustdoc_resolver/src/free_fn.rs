//! Converter for free (non-method) functions.

use std::sync::Arc;

use rustdoc_types::{Item, ItemEnum};

use rustdoc_ext::GlobalItemId;
use rustdoc_ir::{CallableInput, FnHeader, FreeFunction, FreeFunctionPath, RustIdentifier};
use rustdoc_processor::CrateCollection;
use rustdoc_processor::indexing::CrateIndexer;
use rustdoc_processor::queries::Crate;

use crate::errors::*;
use crate::resolve_type::{TypeAliasResolution, resolve_type};

/// Convert a free function from `rustdoc_types` into a [`FreeFunction`].
pub fn resolve_free_function<I: CrateIndexer>(
    item: &Item,
    krate: &Crate,
    krate_collection: &CrateCollection<I>,
    alias_resolution: TypeAliasResolution,
) -> Result<FreeFunction, CallableResolutionError> {
    let ItemEnum::Function(inner) = &item.inner else {
        unreachable!("Expected a function item");
    };

    let canonical_path_segments: Vec<String> =
        krate.import_index.items[&item.id].canonical_path().to_vec();
    // A representation of the path that will be used in error paths
    let path_display = canonical_path_segments.join("::");

    let mut inputs = Vec::new();
    for (parameter_index, (param_name, input_ty)) in inner.sig.inputs.iter().enumerate() {
        match resolve_type(
            input_ty,
            &krate.core.package_id,
            krate_collection,
            &Default::default(),
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
                    callable_path: path_display,
                    callable_item: item.clone(),
                    parameter_type: input_ty.clone(),
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
                &Default::default(),
                alias_resolution,
            ) {
                Ok(t) => Some(t),
                Err(e) => {
                    return Err(OutputTypeResolutionError {
                        callable_path: path_display,
                        callable_item: item.clone(),
                        output_type: output_ty.clone(),
                        source: Arc::new(e.into()),
                    }
                    .into());
                }
            }
        }
        None => None,
    };

    let symbol_name = item.attrs.iter().find_map(|attr| match attr {
        rustdoc_types::Attribute::NoMangle => item.name.clone(),
        rustdoc_types::Attribute::ExportName(name) => Some(name.clone()),
        _ => None,
    });
    let n_segments = canonical_path_segments.len();
    Ok(FreeFunction {
        path: FreeFunctionPath {
            package_id: krate.core.package_id.clone(),
            crate_name: canonical_path_segments[0].clone(),
            module_path: canonical_path_segments[1..n_segments - 1].to_vec(),
            function_name: canonical_path_segments[n_segments - 1].clone(),
            function_generics: vec![],
        },
        header: FnHeader {
            output,
            inputs,
            is_async: inner.header.is_async,
            abi: inner.header.abi.clone(),
            is_unsafe: inner.header.is_unsafe,
            is_c_variadic: inner.sig.is_c_variadic,
            symbol_name,
        },
        source_coordinates: Some(GlobalItemId {
            rustdoc_item_id: item.id,
            package_id: krate.core.package_id.clone(),
        }),
    })
}
