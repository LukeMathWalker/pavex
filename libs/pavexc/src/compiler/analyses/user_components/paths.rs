use super::UserComponent;
use super::UserComponentId;
use super::auxiliary::AuxiliaryData;
use crate::compiler::component::ConfigTypeValidationError;
use crate::diagnostic::CompilerDiagnostic;
use crate::diagnostic::ComponentKind;
use crate::diagnostic::TargetSpan;
use crate::language::{FQPath, ParseError, PathKind};
use crate::{
    compiler::{
        analyses::{computations::ComputationDb, prebuilt_types::PrebuiltTypeDb},
        component::{ConfigType, PrebuiltType, PrebuiltTypeValidationError},
        resolvers::{CallableResolutionError, TypeResolutionError, resolve_type_path},
    },
    diagnostic::CallableDefSource,
};
use crate::{rustdoc::CrateCollection, utils::comma_separated_list};
use guppy::graph::PackageGraph;
use indexmap::IndexMap;

/// Match the id of a component with its fully-qualified path, if there is one.
///
/// # Implementation notes
///
/// We could do this work while we resolve the identifiers directly to
/// types/callables.
/// We isolate it as its own step to be able to pre-determine which crates
/// we need to compute/fetch JSON docs for since we get higher throughput
/// via a batch computation than by computing them one by one as the need
/// arises.
#[derive(Default)]
pub struct FQPaths {
    /// The path associated with each user component.
    id2resolved_path: IndexMap<UserComponentId, FQPath>,
    /// All paths until `resolved_up_to` (excluded) have been resolved to a type/callable.
    ///
    /// This field is used as cursor to avoid performing redundant resolutions if
    /// later stages need to add (and then resolve) new paths.
    resolved_up_to: usize,
    /// All raw identifiers until `fully_qualified_up_to` (excluded) have been
    /// converted into a fully qualified path and add to [`Self::id2resolved_path`]
    /// (if the conversion was successful).
    ///
    /// This field is used as cursor to avoid performing redundant work if
    /// later stages need to process new identifiers.
    fully_qualified_up_to: usize,
}

impl FQPaths {
    /// Create a new instance of [`ResolvedPaths`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Process all identifiers that have been registered with [`AuxiliaryData`].
    pub fn process_identifiers(
        &mut self,
        db: &AuxiliaryData,
        package_graph: &PackageGraph,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
        for (id, _) in db.iter().skip(self.fully_qualified_up_to) {
            self.process_identifier(id, db, package_graph, diagnostics);
        }
        self.fully_qualified_up_to = db.iter().len();
    }

    /// Process a single raw identifier.
    pub fn process_identifier(
        &mut self,
        id: UserComponentId,
        db: &AuxiliaryData,
        package_graph: &PackageGraph,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
        let component = &db[id];
        let Some(identifiers_id) = component.raw_identifiers_id() else {
            return;
        };
        let identifiers = &db.identifiers_interner[identifiers_id];
        let kind = match component.kind() {
            ComponentKind::PrebuiltType | ComponentKind::ConfigType => PathKind::Type,
            ComponentKind::RequestHandler
            | ComponentKind::Fallback
            | ComponentKind::Constructor
            | ComponentKind::ErrorHandler
            | ComponentKind::WrappingMiddleware
            | ComponentKind::PostProcessingMiddleware
            | ComponentKind::PreProcessingMiddleware
            | ComponentKind::ErrorObserver => PathKind::Callable,
        };
        match FQPath::parse(identifiers, package_graph, kind) {
            Ok(path) => {
                self.id2resolved_path.insert(id, path);
            }
            Err(e) => invalid_identifiers(e, id, db, diagnostics),
        }
    }

    /// Return an iterator over the stored paths.
    pub(super) fn values(&self) -> impl Iterator<Item = &FQPath> {
        self.id2resolved_path.values()
    }

    /// Resolve all paths to their corresponding types or callables.
    /// Those items are then interned into the respective databases for later
    /// use.
    #[tracing::instrument(name = "Resolve types and callables", skip_all, level = "trace")]
    pub(super) fn resolve(
        &mut self,
        aux: &mut AuxiliaryData,
        computation_db: &mut ComputationDb,
        prebuilt_type_db: &mut PrebuiltTypeDb,
        krate_collection: &CrateCollection,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
        // First make sure we have processed all newly registered identifiers.
        self.process_identifiers(aux, krate_collection.package_graph(), diagnostics);

        let mut config_id2type = std::mem::take(&mut aux.config_id2type);
        for (id, user_component) in aux.iter().skip(self.resolved_up_to) {
            let Some(resolved_path) = self.id2resolved_path.get(&id) else {
                // Annotated components don't have a resolved path attached to them.
                continue;
            };
            if let UserComponent::PrebuiltType { .. } = &user_component {
                if aux.id2registration[id].kind.is_attribute() {
                    continue;
                }
                match resolve_type_path(resolved_path, krate_collection) {
                    Ok(ty) => match PrebuiltType::new(ty) {
                        Ok(prebuilt) => {
                            prebuilt_type_db.get_or_intern(prebuilt, id);
                        }
                        Err(e) => invalid_prebuilt_type(e, resolved_path, id, aux, diagnostics),
                    },
                    Err(e) => cannot_resolve_type_path(e, id, aux, diagnostics),
                };
            } else if let UserComponent::ConfigType { key, .. } = &user_component {
                if aux.id2registration[id].kind.is_attribute() {
                    continue;
                }
                match resolve_type_path(resolved_path, krate_collection) {
                    Ok(ty) => match ConfigType::new(ty, key.into()) {
                        Ok(config) => {
                            config_id2type.insert(id, config);
                        }
                        Err(e) => invalid_config_type(e, resolved_path, id, aux, diagnostics),
                    },
                    Err(e) => cannot_resolve_type_path(e, id, aux, diagnostics),
                };
            } else if let Err(e) =
                computation_db.resolve_and_intern(krate_collection, resolved_path, Some(id))
            {
                cannot_resolve_callable_path(
                    e,
                    id,
                    aux,
                    krate_collection.package_graph(),
                    diagnostics,
                );
            }
        }
        self.resolved_up_to = self.id2resolved_path.len();
        aux.config_id2type = config_id2type;
    }
}

pub(super) fn invalid_prebuilt_type(
    e: PrebuiltTypeValidationError,
    resolved_path: &FQPath,
    id: UserComponentId,
    db: &AuxiliaryData,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
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
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
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

fn cannot_resolve_type_path(
    e: TypeResolutionError,
    id: UserComponentId,
    db: &AuxiliaryData,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    let component = &db[id];
    let source = diagnostics.annotated(
        TargetSpan::RawIdentifiers(&db.id2registration[id], component.kind()),
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

fn invalid_identifiers(
    e: ParseError,
    id: UserComponentId,
    db: &AuxiliaryData,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    let label_msg = match &e {
        ParseError::InvalidPath(_) => "The invalid path",
        ParseError::PathMustBeAbsolute(_) => "The relative path",
    };
    let source = diagnostics.annotated(db.registration_target(&id), label_msg);
    let help = match &e {
        ParseError::InvalidPath(inner) => {
            inner.raw_path.strip_suffix("()").map(|stripped| format!("The `f!` macro expects an unambiguous path as input, not a function call. Remove the `()` at the end: `f!({stripped})`"))
        }
        ParseError::PathMustBeAbsolute(_) =>
            Some(
                "If it is a local import, the path must start with `crate::`, `self::` or `super::`.\n\
                If it is an import from a dependency, the path must start with \
                the dependency name (e.g. `dependency::`)."
                .to_string(),
            )
    };
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .optional_help(help)
        .build();
    diagnostics.push(diagnostic);
}
