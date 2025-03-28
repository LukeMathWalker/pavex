use ahash::HashMap;
use guppy::graph::PackageGraph;
use syn::spanned::Spanned;

use super::UserComponentId;
use super::{UserComponent, auxiliary::AuxiliaryData};
use crate::diagnostic::TargetSpan;
use crate::{
    compiler::{
        analyses::{
            computations::ComputationDb, config_types::ConfigTypeDb, prebuilt_types::PrebuiltTypeDb,
        },
        component::{
            ConfigType, ConfigTypeValidationError, PrebuiltType, PrebuiltTypeValidationError,
        },
        resolvers::{CallableResolutionError, TypeResolutionError, resolve_type_path},
    },
    diagnostic::{AnnotatedSource, CallableDefinition, CompilerDiagnostic, SourceSpanExt},
};
use crate::{language::ResolvedPath, rustdoc::CrateCollection, utils::comma_separated_list};

/// Resolve all paths to their corresponding types or callables.
/// Those items are then interned into the respective databases for later
/// use.
#[tracing::instrument(name = "Resolve types and callables", skip_all, level = "trace")]
pub(super) fn resolve_paths(
    id2resolved_path: &HashMap<UserComponentId, ResolvedPath>,
    raw_db: &AuxiliaryData,
    computation_db: &mut ComputationDb,
    prebuilt_type_db: &mut PrebuiltTypeDb,
    config_type_db: &mut ConfigTypeDb,
    krate_collection: &CrateCollection,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    for (id, user_component) in raw_db.iter() {
        let Some(resolved_path) = id2resolved_path.get(&id) else {
            // Annotated components don't have a resolved path attached to them.
            continue;
        };
        if let UserComponent::PrebuiltType { .. } = &user_component {
            match resolve_type_path(resolved_path, krate_collection) {
                Ok(ty) => match PrebuiltType::new(ty) {
                    Ok(prebuilt) => {
                        prebuilt_type_db.get_or_intern(prebuilt, id);
                    }
                    Err(e) => invalid_prebuilt_type(e, resolved_path, id, raw_db, diagnostics),
                },
                Err(e) => cannot_resolve_type_path(e, id, raw_db, diagnostics),
            };
        } else if let UserComponent::ConfigType { key, .. } = &user_component {
            match resolve_type_path(resolved_path, krate_collection) {
                Ok(ty) => match ConfigType::new(ty, key.into()) {
                    Ok(config) => {
                        config_type_db.get_or_intern(config, id);
                    }
                    Err(e) => invalid_config_type(e, resolved_path, id, raw_db, diagnostics),
                },
                Err(e) => cannot_resolve_type_path(e, id, raw_db, diagnostics),
            };
        } else if let Err(e) =
            computation_db.resolve_and_intern(krate_collection, resolved_path, Some(id))
        {
            cannot_resolve_callable_path(
                e,
                id,
                raw_db,
                krate_collection.package_graph(),
                diagnostics,
            );
        }
    }
}

fn invalid_prebuilt_type(
    e: PrebuiltTypeValidationError,
    resolved_path: &ResolvedPath,
    id: UserComponentId,
    db: &AuxiliaryData,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    use std::fmt::Write as _;

    let source = diagnostics.annotated(
        TargetSpan::Registration(&db.id2registration[&id]),
        "The prebuilt type was registered here",
    );
    let mut error_msg = e.to_string();
    let help: String = match &e {
        PrebuiltTypeValidationError::CannotHaveLifetimeParameters { ty } => {
            if ty.has_implicit_lifetime_parameters() {
                writeln!(
                        &mut error_msg,
                        "\n`{resolved_path}` has elided lifetime parameters, which might be non-'static."
                    ).unwrap();
            } else {
                let named_lifetimes = ty.named_lifetime_parameters();
                if named_lifetimes.len() == 1 {
                    write!(
                            &mut error_msg,
                            "\n`{resolved_path}` has a named lifetime parameter, `'{}`, that you haven't constrained to be 'static.",
                            named_lifetimes[0]
                        ).unwrap();
                } else {
                    write!(
                            &mut error_msg,
                            "\n`{resolved_path}` has {} named lifetime parameters that you haven't constrained to be 'static: ",
                            named_lifetimes.len(),
                        ).unwrap();
                    comma_separated_list(
                        &mut error_msg,
                        named_lifetimes.iter(),
                        |s| format!("`'{s}`"),
                        "and",
                    )
                    .unwrap();
                    write!(&mut error_msg, ".").unwrap();
                }
            };
            "Set the lifetime parameters to `'static` when registering the type as prebuilt. E.g. `bp.prebuilt(t!(crate::MyType<'static>))` for `struct MyType<'a>(&'a str)`.".to_string()
        }
        PrebuiltTypeValidationError::CannotHaveUnassignedGenericTypeParameters { ty } => {
            let generic_type_parameters = ty.unassigned_generic_type_parameters();
            if generic_type_parameters.len() == 1 {
                write!(
                        &mut error_msg,
                        "\n`{resolved_path}` has a generic type parameter, `{}`, that you haven't assigned a concrete type to.",
                        generic_type_parameters[0]
                    ).unwrap();
            } else {
                write!(
                        &mut error_msg,
                        "\n`{resolved_path}` has {} generic type parameters that you haven't assigned concrete types to: ",
                        generic_type_parameters.len(),
                    ).unwrap();
                comma_separated_list(
                    &mut error_msg,
                    generic_type_parameters.iter(),
                    |s| format!("`{s}`"),
                    "and",
                )
                .unwrap();
                write!(&mut error_msg, ".").unwrap();
            }
            "Set the generic parameters to concrete types when registering the type as prebuilt. E.g. `bp.prebuilt(t!(crate::MyType<std::string::String>))` for `struct MyType<T>(T)`.".to_string()
        }
    };
    let e = anyhow::anyhow!(e).context(error_msg);
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .help(help)
        .build();
    diagnostics.push(diagnostic);
}

fn invalid_config_type(
    e: ConfigTypeValidationError,
    resolved_path: &ResolvedPath,
    id: UserComponentId,
    db: &AuxiliaryData,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    use ConfigTypeValidationError::*;
    use std::fmt::Write as _;

    let registration = &db.id2registration[&id];
    let (target_span, label_msg) = match &e {
        CannotHaveAnyLifetimeParameters { .. }
        | CannotHaveUnassignedGenericTypeParameters { .. } => (
            TargetSpan::Registration(registration),
            "The config type was registered here",
        ),
        InvalidKey { .. } => (
            TargetSpan::ConfigKeySpan(registration),
            "The config key was registered here",
        ),
    };
    let source = diagnostics.annotated(target_span, label_msg);

    let mut error_msg = e.to_string();
    let help: Option<String> = match &e {
        ConfigTypeValidationError::InvalidKey { .. } => None,
        ConfigTypeValidationError::CannotHaveAnyLifetimeParameters { ty } => {
            let named = ty.named_lifetime_parameters();
            let (elided, static_) = ty
                .lifetime_parameters()
                .into_iter()
                .filter(|l| l.is_static() || l.is_elided())
                .partition::<Vec<_>, _>(|l| l.is_elided());
            write!(&mut error_msg, "\n`{resolved_path}` has ").unwrap();
            let mut parts = Vec::new();
            if !named.is_empty() {
                let n = named.len();
                let part = if n == 1 {
                    let lifetime = &named[0];
                    format!("1 named lifetime parameter (`'{lifetime}'`)")
                } else {
                    let mut part = String::new();
                    write!(&mut part, "{n} named lifetime parameters (").unwrap();
                    comma_separated_list(&mut part, named.iter(), |s| format!("`'{s}`"), "and")
                        .unwrap();
                    part.push(')');
                    part
                };
                parts.push(part);
            }
            if !elided.is_empty() {
                let n = elided.len();
                let part = if n == 1 {
                    "1 elided lifetime parameter".to_string()
                } else {
                    format!("{n} elided lifetime parameters")
                };
                parts.push(part);
            }
            if !static_.is_empty() {
                let n = static_.len();
                let part = if n == 1 {
                    "1 static lifetime parameter".to_string()
                } else {
                    format!("{n} static lifetime parameters")
                };
                parts.push(part);
            }
            comma_separated_list(&mut error_msg, parts.iter(), |s| s.to_owned(), "and").unwrap();
            error_msg.push('.');

            Some(
                "Remove all lifetime parameters from the definition of your configuration type."
                    .to_string(),
            )
        }
        ConfigTypeValidationError::CannotHaveUnassignedGenericTypeParameters { ty } => {
            let generic_type_parameters = ty.unassigned_generic_type_parameters();
            if generic_type_parameters.len() == 1 {
                write!(
                            &mut error_msg,
                            "\n`{resolved_path}` has a generic type parameter, `{}`, that you haven't assigned a concrete type to.",
                            generic_type_parameters[0]
                        ).unwrap();
            } else {
                write!(
                            &mut error_msg,
                            "\n`{resolved_path}` has {} generic type parameters that you haven't assigned concrete types to: ",
                            generic_type_parameters.len(),
                        ).unwrap();
                comma_separated_list(
                    &mut error_msg,
                    generic_type_parameters.iter(),
                    |s| format!("`{s}`"),
                    "and",
                )
                .unwrap();
                write!(&mut error_msg, ".").unwrap();
            }
            Some("Set the generic parameters to concrete types when registering the type as configuration. E.g. `bp.config(t!(crate::MyType<std::string::String>))` for `struct MyType<T>(T)`.".to_string())
        }
    };
    let e = anyhow::anyhow!(e).context(error_msg);
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .optional_help(help)
        .build();
    diagnostics.push(diagnostic);
}

fn cannot_resolve_type_path(
    e: TypeResolutionError,
    id: UserComponentId,
    db: &AuxiliaryData,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    let source = diagnostics.annotated(
        TargetSpan::RawIdentifiers(&db.id2registration[&id]),
        "The type that we can't resolve",
    );
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .help("Check that the path is spelled correctly and that the type is public.".into())
        .build();
    diagnostics.push(diagnostic);
}

pub(super) fn cannot_resolve_callable_path(
    e: CallableResolutionError,
    id: UserComponentId,
    db: &AuxiliaryData,
    package_graph: &PackageGraph,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    let component = &db[id];
    let kind = component.kind();
    match e {
        CallableResolutionError::UnknownCallable(_) => {
            let source = diagnostics.annotated(
                TargetSpan::RawIdentifiers(&db.id2registration[&id]),
                format!("The {kind} that we can't resolve"),
            );
            let diagnostic = CompilerDiagnostic::builder(e).optional_source(source)
                    .help("Check that the path is spelled correctly and that the function (or method) is public.".into())
                    .build();
            diagnostics.push(diagnostic);
        }
        CallableResolutionError::InputParameterResolutionError(ref inner_error) => {
            let definition_snippet = match CallableDefinition::compute_from_item(
                &inner_error.callable_item,
                package_graph,
            ) {
                Some(def) => {
                    let mut inputs = def.sig.inputs.iter();
                    let input = inputs.nth(inner_error.parameter_index).cloned().unwrap();
                    let local_span = match input {
                        syn::FnArg::Typed(typed) => typed.ty.span(),
                        syn::FnArg::Receiver(r) => r.span(),
                    };
                    let label = def
                        .convert_local_span(local_span)
                        .labeled("I don't know how handle this parameter".into());
                    Some(AnnotatedSource::new(def.named_source()).label(label))
                }
                _ => None,
            };
            let source = diagnostics.annotated(
                TargetSpan::RawIdentifiers(&db.id2registration[&id]),
                format!("The {kind} was registered here"),
            );
            let diagnostic = CompilerDiagnostic::builder(e.clone())
                .optional_source(source)
                .optional_source(definition_snippet)
                .build();
            diagnostics.push(diagnostic);
        }
        CallableResolutionError::UnsupportedCallableKind(ref inner_error) => {
            let source = diagnostics.annotated(
                TargetSpan::RawIdentifiers(&db.id2registration[&id]),
                format!("It was registered as a {kind} here"),
            );
            let message = format!(
                "I can work with functions and methods, but `{}` is neither.\nIt is {} and I don't know how to use it as a {}.",
                inner_error.import_path, inner_error.item_kind, kind
            );
            let error = anyhow::anyhow!(e).context(message);
            diagnostics.push(
                CompilerDiagnostic::builder(error)
                    .optional_source(source)
                    .build(),
            );
        }
        CallableResolutionError::OutputTypeResolutionError(ref inner_error) => {
            let output_snippet = {
                match CallableDefinition::compute_from_item(
                    &inner_error.callable_item,
                    package_graph,
                ) {
                    Some(def) => match &def.sig.output {
                        syn::ReturnType::Default => None,
                        syn::ReturnType::Type(_, type_) => Some(type_.span()),
                    }
                    .map(|s| {
                        let label = def
                            .convert_local_span(s)
                            .labeled("The output type that I can't handle".into());
                        AnnotatedSource::new(def.named_source()).label(label)
                    }),
                    _ => None,
                }
            };

            let source = diagnostics.annotated(
                TargetSpan::Registration(&db.id2registration[&id]),
                format!("The {kind} was registered here"),
            );
            diagnostics.push(
                CompilerDiagnostic::builder(e.clone())
                    .optional_source(source)
                    .optional_source(output_snippet)
                    .build(),
            )
        }
        CallableResolutionError::CannotGetCrateData(_) => {
            diagnostics.push(CompilerDiagnostic::builder(e).build());
        }
        CallableResolutionError::GenericParameterResolutionError(_) => {
            let source = diagnostics.annotated(
                TargetSpan::Registration(&db.id2registration[&id]),
                format!("The {kind} was registered here"),
            );
            let diagnostic = CompilerDiagnostic::builder(e)
                .optional_source(source)
                .build();
            diagnostics.push(diagnostic);
        }
        CallableResolutionError::SelfResolutionError(_) => {
            let source = diagnostics.annotated(
                TargetSpan::Registration(&db.id2registration[&id]),
                format!("The {kind} was registered here"),
            );
            let diagnostic = CompilerDiagnostic::builder(e)
                .optional_source(source)
                .build();
            diagnostics.push(diagnostic);
        }
    }
}
