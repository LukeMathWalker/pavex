//! Given the fully qualified path to a function (be it a constructor or a handler),
//! find the corresponding item ("resolution") in `rustdoc`'s JSON output to determine
//! its input parameters and output type.
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

use anyhow::anyhow;
use bimap::BiHashMap;
use guppy::graph::PackageGraph;
use guppy::PackageId;
use indexmap::IndexSet;
use miette::{miette, NamedSource, SourceSpan};
use rustdoc_types::{GenericArg, GenericArgs, ItemEnum, Type};
use syn::spanned::Spanned;
use syn::{FnArg, ImplItemMethod, ReturnType};

use pavex_builder::Location;
use pavex_builder::RawCallableIdentifiers;

use crate::diagnostic;
use crate::diagnostic::read_source_file;
use crate::diagnostic::CompilerDiagnostic;
use crate::diagnostic::{
    convert_proc_macro_span, convert_rustdoc_span, LocationExt, OptionalSourceSpanExt,
    SourceSpanExt,
};
use crate::language::{Callable, InvocationStyle, ResolvedPath, ResolvedType, UnknownPath};
use crate::rustdoc::{CannotGetCrateData, RustdocKindExt};
use crate::rustdoc::{CrateCollection, ResolvedItem};

/// Extract the input type paths, the output type path and the callable path for each
/// registered type constructor.
pub(crate) fn resolve_constructors(
    constructor_paths: &IndexSet<ResolvedPath>,
    krate_collection: &CrateCollection,
) -> (
    BiHashMap<ResolvedPath, Callable>,
    Vec<CallableResolutionError>,
) {
    let mut resolution_map = BiHashMap::with_capacity(constructor_paths.len());
    let mut errors = vec![];
    for constructor_identifiers in constructor_paths {
        match resolve_callable(krate_collection, constructor_identifiers) {
            Ok(constructor) => {
                resolution_map.insert(constructor_identifiers.to_owned(), constructor);
            }
            Err(e) => errors.push(e),
        }
    }
    (resolution_map, errors)
}

/// Extract the input type paths, the output type path and the callable path for each
/// registered error handler.
#[allow(clippy::type_complexity)]
pub(crate) fn resolve_error_handlers(
    paths: &IndexSet<ResolvedPath>,
    krate_collection: &CrateCollection,
) -> (
    HashMap<ResolvedPath, Callable>,
    HashMap<Callable, IndexSet<ResolvedPath>>,
    Vec<CallableResolutionError>,
) {
    let mut resolution_map = HashMap::with_capacity(paths.len());
    let mut reverse_map = HashMap::<Callable, IndexSet<ResolvedPath>>::new();
    let mut errors = vec![];
    for path in paths {
        match resolve_callable(krate_collection, path) {
            Ok(callable) => {
                resolution_map.insert(path.to_owned(), callable.clone());
                reverse_map
                    .entry(callable)
                    .or_default()
                    .insert(path.to_owned());
            }
            Err(e) => errors.push(e),
        }
    }
    (resolution_map, reverse_map, errors)
}

/// Extract the input type paths, the output type path and the callable path for each
/// registered request handler.
pub(crate) fn resolve_request_handlers(
    handler_paths: &IndexSet<ResolvedPath>,
    krate_collection: &CrateCollection,
) -> (
    HashMap<ResolvedPath, Callable>,
    HashMap<Callable, IndexSet<ResolvedPath>>,
    IndexSet<Callable>,
    Vec<CallableResolutionError>,
) {
    let mut handlers = IndexSet::with_capacity(handler_paths.len());
    let mut handler_resolver = HashMap::new();
    let mut reverse_map = HashMap::<Callable, IndexSet<ResolvedPath>>::new();
    let mut errors = vec![];
    for callable_path in handler_paths {
        match resolve_callable(krate_collection, callable_path) {
            Ok(handler) => {
                handlers.insert(handler.clone());
                handler_resolver.insert(callable_path.to_owned(), handler.clone());
                reverse_map
                    .entry(handler)
                    .or_default()
                    .insert(callable_path.to_owned());
            }
            Err(e) => errors.push(e),
        }
    }
    (handler_resolver, reverse_map, handlers, errors)
}

pub(crate) fn resolve_type(
    type_: &Type,
    // The package id where the type we are trying to process has been referenced (e.g. as an
    // input/output parameter).
    used_by_package_id: &PackageId,
    krate_collection: &CrateCollection,
    generic_bindings: &HashMap<String, ResolvedType>,
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
                                    generics.push(resolve_type(
                                        generic_type,
                                        used_by_package_id,
                                        krate_collection,
                                        generic_bindings,
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
            let (global_type_id, base_type) =
                krate_collection.get_canonical_path_by_local_type_id(used_by_package_id, id)?;
            Ok(ResolvedType {
                package_id: global_type_id.package_id().to_owned(),
                rustdoc_id: Some(global_type_id.rustdoc_item_id),
                base_type: base_type.to_vec(),
                generic_arguments: generics,
                is_shared_reference: false,
            })
        }
        Type::BorrowedRef {
            lifetime: _,
            mutable,
            type_,
        } => {
            if *mutable {
                return Err(anyhow!(
                    "Mutable references are not allowed. You can only pass an argument \
                    by value (`move` semantic) or via a shared reference (`&MyType`)",
                ));
            }
            let mut resolved_type = resolve_type(
                type_,
                used_by_package_id,
                krate_collection,
                generic_bindings,
            )?;
            resolved_type.is_shared_reference = true;
            Ok(resolved_type)
        }
        Type::Generic(s) => {
            if let Some(resolved_type) = generic_bindings.get(s) {
                Ok(resolved_type.to_owned())
            } else {
                Err(anyhow!(
                    "The generic type `{}` is not bound to any concrete type",
                    s
                ))
            }
        }
        _ => Err(anyhow!(
            "I cannot handle this kind ({:?}) of type yet. Sorry!",
            type_
        )),
    }
}

pub(crate) fn resolve_callable(
    krate_collection: &CrateCollection,
    callable_path: &ResolvedPath,
) -> Result<Callable, CallableResolutionError> {
    let (callable_type, qualified_self_type) =
        callable_path.find_rustdoc_items(krate_collection)?;
    let used_by_package_id = &callable_path.package_id;
    let (header, decl, invocation_style) = match &callable_type.item.item.inner {
        ItemEnum::Function(f) => (&f.header, &f.decl, InvocationStyle::FunctionCall),
        kind => {
            let item_kind = kind.kind().to_owned();
            return Err(UnsupportedCallableKind {
                import_path: callable_path.to_owned(),
                item_kind,
            }
            .into());
        }
    };

    let mut generic_bindings = HashMap::new();
    if let Some(qself) = qualified_self_type {
        let qself_path = &callable_path.qualified_self.as_ref().unwrap().path;
        let qself_type = resolve_type_path(&qself_path, &qself.item, krate_collection).unwrap();
        generic_bindings.insert("Self".to_string(), qself_type);
    }

    let mut parameter_paths = Vec::with_capacity(decl.inputs.len());
    for (parameter_index, (_, parameter_type)) in decl.inputs.iter().enumerate() {
        match resolve_type(
            parameter_type,
            used_by_package_id,
            krate_collection,
            &generic_bindings,
        ) {
            Ok(p) => parameter_paths.push(p),
            Err(e) => {
                return Err(ParameterResolutionError {
                    parameter_type: parameter_type.to_owned(),
                    callable_path: callable_path.to_owned(),
                    callable_item: callable_type.item.item.into_owned(),
                    source: e,
                    parameter_index,
                }
                .into());
            }
        }
    }
    let output_type_path = match &decl.output {
        // Unit type
        None => None,
        Some(output_type) => {
            match resolve_type(
                output_type,
                used_by_package_id,
                krate_collection,
                &generic_bindings,
            ) {
                Ok(p) => Some(p),
                Err(e) => {
                    return Err(OutputTypeResolutionError {
                        output_type: output_type.to_owned(),
                        callable_path: callable_path.to_owned(),
                        callable_item: callable_type.item.item.into_owned(),
                        source: e,
                    }
                    .into());
                }
            }
        }
    };
    let callable = Callable {
        is_async: header.async_,
        output: output_type_path,
        path: callable_path.to_owned(),
        inputs: parameter_paths,
        invocation_style,
    };
    Ok(callable)
}

pub(crate) fn resolve_type_path(
    path: &ResolvedPath,
    resolved_item: &ResolvedItem,
    krate_collection: &CrateCollection,
) -> Result<ResolvedType, anyhow::Error> {
    let item = &resolved_item.item;
    let used_by_package_id = resolved_item.item_id.package_id();
    let (global_type_id, base_type) =
        krate_collection.get_canonical_path_by_local_type_id(used_by_package_id, &item.id)?;
    let mut generic_arguments = vec![];
    for segment in &path.segments {
        for generic_path in &segment.generic_arguments {
            let (generic_item, generic_qself_item) =
                generic_path.find_rustdoc_items(krate_collection)?;
            assert!(generic_qself_item.is_none());
            let generic_type =
                resolve_type_path(generic_path, &generic_item.item, krate_collection)?;
            generic_arguments.push(generic_type);
        }
    }
    Ok(ResolvedType {
        package_id: global_type_id.package_id().to_owned(),
        rustdoc_id: Some(global_type_id.rustdoc_item_id),
        base_type: base_type.to_vec(),
        generic_arguments,
        is_shared_reference: false,
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

#[derive(Debug, Copy, Clone)]
pub(crate) enum CallableType {
    RequestHandler,
    Constructor,
    ErrorHandler,
}

impl Display for CallableType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CallableType::RequestHandler => "request handler",
            CallableType::Constructor => "constructor",
            CallableType::ErrorHandler => "error handler",
        };
        write!(f, "{}", s)
    }
}

impl CallableResolutionError {
    pub(crate) fn into_diagnostic<LocationProvider>(
        self,
        resolved_paths2identifiers: &HashMap<ResolvedPath, HashSet<RawCallableIdentifiers>>,
        identifiers2location: LocationProvider,
        package_graph: &PackageGraph,
        callable_type: CallableType,
    ) -> Result<CompilerDiagnostic, miette::Error>
    where
        LocationProvider: Fn(&RawCallableIdentifiers) -> Location,
    {
        match self {
            Self::UnknownCallable(e) => {
                // We only report a single registration site in the error report even though
                // the same callable might have been registered in multiple locations.
                // We may or may not want to change this in the future.
                let type_path = &e.0;
                let raw_identifier = resolved_paths2identifiers[type_path].iter().next().unwrap();
                let location = identifiers2location(raw_identifier);
                let source = location.source_file(&package_graph)?;
                let label = diagnostic::get_f_macro_invocation_span(&source, &location)
                    .map(|s| s.labeled(format!("The {callable_type} that we cannot resolve")));
                Ok(CompilerDiagnostic::builder(source, e)
                    .optional_label(label)
                    .help("This is most likely a bug in `pavex` or `rustdoc`.\nPlease file a GitHub issue!".into())
                    .build())
            }
            CallableResolutionError::ParameterResolutionError(e) => {
                let sub_diagnostic = {
                    if let Some(definition_span) = &e.callable_item.span {
                        let source_contents =
                            read_source_file(&definition_span.filename, &package_graph.workspace())
                                .map_err(miette::MietteError::IoError)?;
                        let span =
                            convert_rustdoc_span(&source_contents, definition_span.to_owned());
                        let span_contents =
                            &source_contents[span.offset()..(span.offset() + span.len())];
                        let input = match &e.callable_item.inner {
                            ItemEnum::Function(_) => {
                                if let Ok(item) = syn::parse_str::<syn::ItemFn>(span_contents) {
                                    let mut inputs = item.sig.inputs.iter();
                                    inputs.nth(e.parameter_index).cloned()
                                } else if let Ok(item) =
                                    syn::parse_str::<ImplItemMethod>(span_contents)
                                {
                                    let mut inputs = item.sig.inputs.iter();
                                    inputs.nth(e.parameter_index).cloned()
                                } else {
                                    panic!(
                                        "Could not parse as a function or method:\n{span_contents}"
                                    )
                                }
                            }
                            _ => unreachable!(),
                        }
                        .unwrap();
                        let s = convert_proc_macro_span(
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
                        .labeled("I do not know how handle this parameter".into());
                        let source_code = NamedSource::new(
                            definition_span.filename.to_str().unwrap(),
                            source_contents,
                        );
                        Some(
                            CompilerDiagnostic::builder(source_code, anyhow::anyhow!(""))
                                .label(label)
                                .build(),
                        )
                    } else {
                        None
                    }
                };

                let callable_path = &e.callable_path;
                let raw_identifier = resolved_paths2identifiers[callable_path]
                    .iter()
                    .next()
                    .unwrap();
                let location = identifiers2location(raw_identifier);
                let source = location.source_file(&package_graph)?;
                let label = diagnostic::get_f_macro_invocation_span(&source, &location)
                    .map(|s| s.labeled(format!("The {callable_type} was registered here")));
                Ok(CompilerDiagnostic::builder(source, e)
                    .optional_label(label)
                    .optional_related_error(sub_diagnostic)
                    .build())
            }
            CallableResolutionError::UnsupportedCallableKind(e) => {
                let type_path = &e.import_path;
                let raw_identifier = resolved_paths2identifiers[type_path].iter().next().unwrap();
                let location = identifiers2location(raw_identifier);
                let source = location.source_file(&package_graph)?;
                let label = diagnostic::get_f_macro_invocation_span(&source, &location)
                    .map(|s| s.labeled(format!("It was registered as a {callable_type} here")));
                Ok(CompilerDiagnostic::builder(source, e)
                    .optional_label(label)
                    .build())
            }
            CallableResolutionError::OutputTypeResolutionError(e) => {
                let sub_diagnostic = {
                    if let Some(definition_span) = &e.callable_item.span {
                        let source_contents =
                            read_source_file(&definition_span.filename, &package_graph.workspace())
                                .map_err(miette::MietteError::IoError)?;
                        let span =
                            convert_rustdoc_span(&source_contents, definition_span.to_owned());
                        let span_contents =
                            &source_contents[span.offset()..(span.offset() + span.len())];
                        let output = match &e.callable_item.inner {
                            ItemEnum::Function(_) => {
                                if let Ok(item) = syn::parse_str::<syn::ItemFn>(span_contents) {
                                    item.sig.output
                                } else if let Ok(item) =
                                    syn::parse_str::<syn::ImplItemMethod>(span_contents)
                                {
                                    item.sig.output
                                } else {
                                    panic!(
                                        "Could not parse as a function or method:\n{span_contents}"
                                    )
                                }
                            }
                            _ => unreachable!(),
                        };
                        let source_span = match output {
                            ReturnType::Default => None,
                            ReturnType::Type(_, type_) => Some(type_.span()),
                        }
                        .map(|s| {
                            let s = convert_proc_macro_span(span_contents, s);
                            SourceSpan::new(
                                // We must shift the offset forward because it's the
                                // offset from the beginning of the file slice that
                                // we deserialized, instead of the entire file
                                (s.offset() + span.offset()).into(),
                                s.len().into(),
                            )
                        });
                        let label =
                            source_span.labeled("The output type that I cannot handle".into());
                        let source_code = NamedSource::new(
                            definition_span.filename.to_str().unwrap(),
                            source_contents,
                        );
                        Some(
                            CompilerDiagnostic::builder(source_code, anyhow::anyhow!(""))
                                .optional_label(label)
                                .build(),
                        )
                    } else {
                        None
                    }
                };

                let callable_path = &e.callable_path;
                let raw_identifier = resolved_paths2identifiers[callable_path]
                    .iter()
                    .next()
                    .unwrap();
                let location = identifiers2location(raw_identifier);
                let source = location.source_file(&package_graph)?;
                let label = diagnostic::get_f_macro_invocation_span(&source, &location)
                    .map(|s| s.labeled(format!("The {callable_type} was registered here")));
                Ok(CompilerDiagnostic::builder(source, e)
                    .optional_label(label)
                    .optional_related_error(sub_diagnostic)
                    .build())
            }
            CallableResolutionError::CannotGetCrateData(e) => Err(miette!(e)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("I can work with functions and static methods, but `{import_path}` is neither.\nIt is {item_kind} and I do not know how to handle it here.")]
pub(crate) struct UnsupportedCallableKind {
    pub import_path: ResolvedPath,
    pub item_kind: String,
}

#[derive(Debug, thiserror::Error)]
#[error("One of the input parameters for `{callable_path}` has a type that I cannot handle.")]
pub(crate) struct ParameterResolutionError {
    pub callable_path: ResolvedPath,
    pub callable_item: rustdoc_types::Item,
    pub parameter_type: Type,
    pub parameter_index: usize,
    #[source]
    pub source: anyhow::Error,
}

#[derive(Debug, thiserror::Error)]
#[error("I do not know how to handle the type returned by `{callable_path}`.")]
pub(crate) struct OutputTypeResolutionError {
    pub callable_path: ResolvedPath,
    pub callable_item: rustdoc_types::Item,
    pub output_type: Type,
    #[source]
    pub source: anyhow::Error,
}
