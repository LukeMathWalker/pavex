use crate::{
    compiler::{
        analyses::user_components::{UserComponentId, imports::UnresolvedImport},
        component::{ConfigTypeValidationError, PrebuiltTypeValidationError},
    },
    diagnostic::{
        self, CallableDefSource, ComponentKind, DiagnosticSink, OptionalLabeledSpanExt,
        OptionalSourceSpanExt, Registration, TargetSpan,
    },
    utils::comma_separated_list,
};
use guppy::graph::PackageGraph;
use pavex_cli_diagnostic::CompilerDiagnostic;
use rustdoc_types::Item;

use super::{AuxiliaryData, CallableResolutionError, ConstGenericsAreNotSupported, FQPath};

pub(super) fn const_generics_are_not_supported(
    e: ConstGenericsAreNotSupported,
    item: &Item,
    diagnostics: &DiagnosticSink,
) {
    let source = item.span.as_ref().and_then(|s| {
        diagnostics.annotated(
            TargetSpan::Registration(&Registration::attribute(s), ComponentKind::Constructor),
            "The annotated item",
        )
    });
    let const_name = e.name;
    let err_msg = match &item.name {
        Some(name) => {
            format!(
                "Pavex does not support const generics.\n`{name}` uses at least one const generic parameter, named `{const_name}`.",
            )
        }
        None => format!(
            "Pavex does not support const generics.\nOne of your types uses at least one const generic parameter, named `{const_name}`."
        ),
    };
    let diagnostic = CompilerDiagnostic::builder(anyhow::anyhow!(err_msg))
        .optional_source(source)
        .help("Remove the const generic parameter from your type definition, or use a different type.".into())
        .build();
    diagnostics.push(diagnostic);
}

pub(super) fn not_a_type_reexport(
    use_item: &Item,
    imported_item_kind: &str,
    component_kind: ComponentKind,
    diagnostics: &DiagnosticSink,
) {
    let source = use_item.span.as_ref().and_then(|s| {
        diagnostics.annotated(
            TargetSpan::Registration(&Registration::attribute(s), component_kind),
            "The annotated re-export",
        )
    });
    let e = anyhow::anyhow!(
        "You can't register {imported_item_kind} as a {component_kind}.\n\
        The re-exported item must be either an enum or a struct."
    );
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .build();
    diagnostics.push(diagnostic);
}

pub(super) fn unresolved_external_reexport(
    use_item: &Item,
    component_kind: ComponentKind,
    diagnostics: &DiagnosticSink,
) {
    let source = use_item.span.as_ref().and_then(|s| {
        diagnostics.annotated(
            TargetSpan::Registration(&Registration::attribute(s), component_kind),
            "The annotated re-export",
        )
    });
    let e = anyhow::anyhow!("I can't find the definition for the re-exported item.");
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .help("Are you sure that the re-exported item is an enum or a struct?".into())
        .build();
    diagnostics.push(diagnostic);
}

pub(super) fn unknown_module_path(
    module_path: &[String],
    krate_name: &str,
    import: &UnresolvedImport,
    diagnostics: &DiagnosticSink,
) {
    let source = diagnostics.source(&import.registered_at).map(|s| {
        diagnostic::imported_sources_span(s.source(), &import.registered_at)
            .labeled("The import was registered here".into())
            .attach(s)
    });
    let module_path = module_path.join("::");
    let e = anyhow::anyhow!(
        "You tried to import from `{module_path}`, but there is no module with that path in `{krate_name}`."
    );
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .build();
    diagnostics.push(diagnostic);
}

pub(super) fn not_a_module(
    path: &[String],
    import: &UnresolvedImport,
    diagnostics: &DiagnosticSink,
) {
    let source = diagnostics.source(&import.registered_at).map(|s| {
        diagnostic::imported_sources_span(s.source(), &import.registered_at)
            .labeled("The import was registered here".into())
            .attach(s)
    });
    let path = path.join("::");
    let e = anyhow::anyhow!("You tried to import from `{path}`, but `{path}` is not a module.");
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .help(
            "Specify the path of the module that contains the item you want to import, rather than the actual item path.".into()
        )
        .build();
    diagnostics.push(diagnostic);
}

pub(super) fn invalid_prebuilt_type(
    e: PrebuiltTypeValidationError,
    resolved_path: &FQPath,
    id: UserComponentId,
    db: &AuxiliaryData,
    diagnostics: &crate::diagnostic::DiagnosticSink,
) {
    use std::fmt::Write as _;

    let source = diagnostics.annotated(
        db.registration_target(&id),
        "The prebuilt type was registered here",
    );
    let mut error_msg = e.to_string();
    let help: String = match &e {
        PrebuiltTypeValidationError::CannotHaveLifetimeParameters { ty } => {
            if ty.has_implicit_lifetime_parameters() {
                writeln!(
                    &mut error_msg,
                    "\n`{resolved_path}` has elided lifetime parameters."
                )
                .unwrap();
            } else {
                let named_lifetimes = ty.named_lifetime_parameters();
                if named_lifetimes.len() == 1 {
                    write!(
                        &mut error_msg,
                        "\n`{resolved_path}` has a named lifetime parameter, `'{}`.",
                        named_lifetimes[0]
                    )
                    .unwrap();
                } else {
                    write!(
                        &mut error_msg,
                        "\n`{resolved_path}` has {} named lifetime parameters: ",
                        named_lifetimes.len(),
                    )
                    .unwrap();
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

            "Remove all lifetime parameters from the definition of your configuration type."
                .to_string()
        }
        PrebuiltTypeValidationError::CannotHaveUnassignedGenericTypeParameters { ty } => {
            let generic_type_parameters = ty.unassigned_generic_type_parameters();
            if generic_type_parameters.len() == 1 {
                write!(
                    &mut error_msg,
                    "\n`{resolved_path}` has a generic type parameter, `{}`.",
                    generic_type_parameters[0]
                )
                .unwrap();
            } else {
                write!(
                    &mut error_msg,
                    "\n`{resolved_path}` has {} generic type parameters: ",
                    generic_type_parameters.len(),
                )
                .unwrap();
                comma_separated_list(
                    &mut error_msg,
                    generic_type_parameters.iter(),
                    |s| format!("`{s}`"),
                    "and",
                )
                .unwrap();
                write!(&mut error_msg, ".").unwrap();
            }
            "Remove all generic type parameters from the definition of your configuration type."
                .into()
        }
    };
    let e = anyhow::anyhow!(e).context(error_msg);
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .help(help)
        .build();
    diagnostics.push(diagnostic);
}

pub(super) fn invalid_config_type(
    e: ConfigTypeValidationError,
    resolved_path: &FQPath,
    id: UserComponentId,
    db: &AuxiliaryData,
    diagnostics: &crate::diagnostic::DiagnosticSink,
) {
    use ConfigTypeValidationError::*;
    use std::fmt::Write as _;

    let registration = &db.id2registration[id];
    let (target_span, label_msg) = match &e {
        CannotHaveAnyLifetimeParameters { .. }
        | CannotHaveUnassignedGenericTypeParameters { .. } => (
            db.registration_target(&id),
            "The config type was registered here",
        ),
        InvalidKey { .. } => (
            TargetSpan::ConfigKeySpan(registration),
            "The config key was specified here",
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
            let part = if !named.is_empty() {
                let n = named.len();
                if n == 1 {
                    let lifetime = &named[0];
                    format!("1 named lifetime parameter, `{lifetime}`")
                } else {
                    let mut part = String::new();
                    write!(&mut part, "{n} named lifetime parameters: ").unwrap();
                    comma_separated_list(&mut part, named.iter(), |s| format!("`'{s}`"), "and")
                        .unwrap();
                    part
                }
            } else if !elided.is_empty() {
                let n = elided.len();
                if n == 1 {
                    "1 elided lifetime parameter".to_string()
                } else {
                    format!("{n} elided lifetime parameters")
                }
            } else if !static_.is_empty() {
                let n = static_.len();
                if n == 1 {
                    "1 static lifetime parameter".to_string()
                } else {
                    format!("{n} static lifetime parameters")
                }
            } else {
                unreachable!()
            };
            error_msg.push_str(&part);
            error_msg.push('.');

            Some(
                "Remove all lifetime parameters from the definition of your configuration type."
                    .to_string(),
            )
        }
        ConfigTypeValidationError::CannotHaveUnassignedGenericTypeParameters { ty } => {
            if registration.kind.is_attribute() {
                Some(
                    "Remove all generic type parameters from the definition of your configuration type.".into()
                )
            } else {
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
        }
    };
    let e = anyhow::anyhow!(e).context(error_msg);
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .optional_help(help)
        .build();
    diagnostics.push(diagnostic);
}

pub(super) fn cannot_resolve_callable_path(
    e: CallableResolutionError,
    id: UserComponentId,
    db: &AuxiliaryData,
    package_graph: &PackageGraph,
    diagnostics: &crate::diagnostic::DiagnosticSink,
) {
    let component = &db[id];
    let kind = component.kind();
    match e {
        CallableResolutionError::UnknownCallable(_) => {
            let source = diagnostics.annotated(
                TargetSpan::RawIdentifiers(&db.id2registration[id], kind),
                format!("The {kind} that we can't resolve"),
            );
            let diagnostic = CompilerDiagnostic::builder(e).optional_source(source)
                    .help("Check that the path is spelled correctly and that the function (or method) is public.".into())
                    .build();
            diagnostics.push(diagnostic);
        }
        CallableResolutionError::InputParameterResolutionError(ref inner_error) => {
            let definition_snippet =
                CallableDefSource::compute_from_item(&inner_error.callable_item, package_graph)
                    .map(|mut def| {
                        def.label_input(
                            inner_error.parameter_index,
                            "I don't know how handle this parameter",
                        );
                        def.annotated_source
                    });
            let source = diagnostics.annotated(
                TargetSpan::RawIdentifiers(&db.id2registration[id], kind),
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
                TargetSpan::RawIdentifiers(&db.id2registration[id], kind),
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
            let output_snippet =
                CallableDefSource::compute_from_item(&inner_error.callable_item, package_graph)
                    .map(|mut def| {
                        def.label_output("The output type that I can't handle");
                        def.annotated_source
                    });

            let source = diagnostics.annotated(
                db.registration_target(&id),
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
                db.registration_target(&id),
                format!("The {kind} was registered here"),
            );
            let diagnostic = CompilerDiagnostic::builder(e)
                .optional_source(source)
                .build();
            diagnostics.push(diagnostic);
        }
        CallableResolutionError::SelfResolutionError(_) => {
            let source = diagnostics.annotated(
                db.registration_target(&id),
                format!("The {kind} was registered here"),
            );
            let diagnostic = CompilerDiagnostic::builder(e)
                .optional_source(source)
                .build();
            diagnostics.push(diagnostic);
        }
    }
}
