use crate::compiler::analyses::components::ComponentDb;
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::user_components::{UserComponentDb, UserComponentId};
use crate::compiler::component::{
    ConstructorValidationError, ErrorHandlerValidationError, ErrorObserverValidationError,
    RequestHandlerValidationError, WrappingMiddlewareValidationError,
};
use crate::compiler::resolvers::CallableResolutionError;
use crate::compiler::traits::MissingTraitImplementationError;
use crate::diagnostic::{
    convert_proc_macro_span, convert_rustdoc_span, AnnotatedSnippet, CallableDefinition,
    CallableType, CompilerDiagnostic, OptionalSourceSpanExt, SourceSpanExt,
};
use crate::language::{Callable, ResolvedType};
use crate::rustdoc::CrateCollection;
use crate::utils::comma_separated_list;
use crate::{diagnostic, source_or_exit_with_error};
use guppy::graph::PackageGraph;
use indexmap::IndexSet;
use miette::NamedSource;
use rustdoc_types::ItemEnum;
use syn::spanned::Spanned;

/// Utility functions to produce diagnostics.
impl ComponentDb {
    pub(super) fn invalid_constructor(
        e: ConstructorValidationError,
        user_component_id: UserComponentId,
        user_component_db: &UserComponentDb,
        computation_db: &ComputationDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = user_component_db.get_location(user_component_id);
        let source = source_or_exit_with_error!(location, package_graph, diagnostics);
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .labeled("The constructor was registered here".into());
        let diagnostic = match e {
            ConstructorValidationError::CannotFalliblyReturnTheUnitType
            | ConstructorValidationError::CannotConstructPavexError
            | ConstructorValidationError::CannotConstructFrameworkPrimitive { .. }
            | ConstructorValidationError::CannotReturnTheUnitType => {
                CompilerDiagnostic::builder(source, e)
                    .optional_label(label)
                    .build()
            }
            ConstructorValidationError::UnderconstrainedGenericParameters { ref parameters } => {
                fn get_definition_span(
                    callable: &Callable,
                    free_parameters: &IndexSet<String>,
                    krate_collection: &CrateCollection,
                    package_graph: &PackageGraph,
                ) -> Option<AnnotatedSnippet> {
                    let global_item_id = callable.source_coordinates.as_ref()?;
                    let item = krate_collection.get_type_by_global_type_id(global_item_id);
                    let definition_span = item.span.as_ref()?;
                    let source_contents = diagnostic::read_source_file(
                        &definition_span.filename,
                        &package_graph.workspace(),
                    )
                    .ok()?;
                    let span = convert_rustdoc_span(&source_contents, definition_span.to_owned());
                    let span_contents =
                        source_contents[span.offset()..(span.offset() + span.len())].to_string();
                    let (generic_params, output) = match &item.inner {
                        ItemEnum::Function(_) => {
                            if let Ok(item) = syn::parse_str::<syn::ItemFn>(&span_contents) {
                                (item.sig.generics.params, item.sig.output)
                            } else if let Ok(item) =
                                syn::parse_str::<syn::ImplItemFn>(&span_contents)
                            {
                                (item.sig.generics.params, item.sig.output)
                            } else {
                                panic!("Could not parse as a function or method:\n{span_contents}")
                            }
                        }
                        _ => unreachable!(),
                    };

                    let mut labels = vec![];
                    let subject_verb = if generic_params.len() == 1 {
                        "it is"
                    } else {
                        "they are"
                    };
                    for param in generic_params {
                        if let syn::GenericParam::Type(ty) = param {
                            if free_parameters.contains(ty.ident.to_string().as_str()) {
                                labels.push(
                                    convert_proc_macro_span(&span_contents, ty.span())
                                        .labeled("I can't infer this..".into()),
                                );
                            }
                        }
                    }
                    let output_span = if let syn::ReturnType::Type(_, output_type) = &output {
                        output_type.span()
                    } else {
                        output.span()
                    };
                    labels.push(
                        convert_proc_macro_span(&span_contents, output_span)
                            .labeled(format!("..because {subject_verb} not used here")),
                    );
                    let source_path = definition_span.filename.to_str().unwrap();
                    Some(AnnotatedSnippet::new_with_labels(
                        NamedSource::new(source_path, span_contents),
                        labels,
                    ))
                }

                let callable = &computation_db[user_component_id];
                let definition_snippet =
                    get_definition_span(callable, parameters, krate_collection, package_graph);
                let subject_verb = if parameters.len() == 1 {
                    "it is"
                } else {
                    "they are"
                };
                let free_parameters = if parameters.len() == 1 {
                    format!("`{}`", &parameters[0])
                } else {
                    let mut buffer = String::new();
                    comma_separated_list(
                        &mut buffer,
                        parameters.iter(),
                        |p| format!("`{}`", p),
                        "and",
                    )
                    .unwrap();
                    buffer
                };
                let error = anyhow::anyhow!(e)
                    .context(
                        format!(
                            "I am not smart enough to figure out the concrete type for all the generic parameters in `{}`.\n\
                            I can only infer the type of an unassigned generic parameter if it appears in the output type returned by the constructor. This is \
                            not the case for {free_parameters}, since {subject_verb} only used by the input parameters.",
                            callable.path));
                CompilerDiagnostic::builder(source, error)
                    .optional_label(label)
                    .optional_additional_annotated_snippet(definition_snippet)
                    .help(
                        "Specify the concrete type(s) for the problematic \
                        generic parameter(s) when registering the constructor against the blueprint: \n\
                        |  bp.constructor(\n\
                        |    f!(my_crate::my_constructor::<ConcreteType>), \n\
                        |    ..\n\
                        |  )".into())
                    // ^ TODO: add a proper code snippet here, using the actual function that needs
                    //    to be amended instead of a made signature
                    .build()
            }
            ConstructorValidationError::NakedGenericOutputType {
                ref naked_parameter,
            } => {
                fn get_definition_span(
                    callable: &Callable,
                    krate_collection: &CrateCollection,
                    package_graph: &PackageGraph,
                ) -> Option<AnnotatedSnippet> {
                    let global_item_id = callable.source_coordinates.as_ref()?;
                    let item = krate_collection.get_type_by_global_type_id(global_item_id);
                    let definition_span = item.span.as_ref()?;
                    let source_contents = diagnostic::read_source_file(
                        &definition_span.filename,
                        &package_graph.workspace(),
                    )
                    .ok()?;
                    let span = convert_rustdoc_span(&source_contents, definition_span.to_owned());
                    let span_contents =
                        source_contents[span.offset()..(span.offset() + span.len())].to_string();
                    let output = match &item.inner {
                        ItemEnum::Function(_) => {
                            if let Ok(item) = syn::parse_str::<syn::ItemFn>(&span_contents) {
                                item.sig.output
                            } else if let Ok(item) =
                                syn::parse_str::<syn::ImplItemFn>(&span_contents)
                            {
                                item.sig.output
                            } else {
                                panic!("Could not parse as a function or method:\n{span_contents}")
                            }
                        }
                        _ => unreachable!(),
                    };

                    let output_span = if let syn::ReturnType::Type(_, output_type) = &output {
                        output_type.span()
                    } else {
                        output.span()
                    };
                    let label = convert_proc_macro_span(&span_contents, output_span)
                        .labeled("The invalid output type".to_string());
                    let source_path = definition_span.filename.to_str().unwrap();
                    Some(AnnotatedSnippet::new(
                        NamedSource::new(source_path, span_contents),
                        label,
                    ))
                }

                let callable = &computation_db[user_component_id];
                let definition_snippet =
                    get_definition_span(callable, krate_collection, package_graph);
                let msg = format!(
                    "You can't return a naked generic parameter from a constructor, like `{naked_parameter}` in `{}`.\n\
                    I don't take into account trait bounds when building your dependency graph. A constructor \
                    that returns a naked generic parameter is equivalent, in my eyes, to a constructor that can build \
                    **any** type, which is unlikely to be what you want!",
                    callable.path
                );
                let error = anyhow::anyhow!(e).context(msg);
                CompilerDiagnostic::builder(source, error)
                    .optional_label(label)
                    .optional_additional_annotated_snippet(definition_snippet)
                    .help(
                        "Can you return a concrete type as output? \n\
                        Or wrap the generic parameter in a non-generic container? \
                        For example, `T` in `Vec<T>` is not considered to be a naked parameter."
                            .into(),
                    )
                    .build()
            }
        };
        diagnostics.push(diagnostic.into());
    }

    pub(super) fn invalid_request_handler(
        e: RequestHandlerValidationError,
        user_component_id: UserComponentId,
        user_component_db: &UserComponentDb,
        computation_db: &ComputationDb,
        krate_collection: &CrateCollection,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = user_component_db.get_location(user_component_id);
        let source = source_or_exit_with_error!(location, package_graph, diagnostics);
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .labeled("The request handler was registered here".into());
        let diagnostic = match e {
            RequestHandlerValidationError::CannotReturnTheUnitType
            | RequestHandlerValidationError::CannotFalliblyReturnTheUnitType => {
                CompilerDiagnostic::builder(source, e)
                    .optional_label(label)
                    .build()
            }
            RequestHandlerValidationError::UnderconstrainedGenericParameters { ref parameters } => {
                fn get_definition_span(
                    callable: &Callable,
                    free_parameters: &IndexSet<String>,
                    krate_collection: &CrateCollection,
                    package_graph: &PackageGraph,
                ) -> Option<AnnotatedSnippet> {
                    let global_item_id = callable.source_coordinates.as_ref()?;
                    let item = krate_collection.get_type_by_global_type_id(global_item_id);
                    let definition_span = item.span.as_ref()?;
                    let source_contents = diagnostic::read_source_file(
                        &definition_span.filename,
                        &package_graph.workspace(),
                    )
                    .ok()?;
                    let span = convert_rustdoc_span(&source_contents, definition_span.to_owned());
                    let span_contents =
                        source_contents[span.offset()..(span.offset() + span.len())].to_string();
                    let generic_params = match &item.inner {
                        ItemEnum::Function(_) => {
                            if let Ok(item) = syn::parse_str::<syn::ItemFn>(&span_contents) {
                                item.sig.generics.params
                            } else if let Ok(item) =
                                syn::parse_str::<syn::ImplItemFn>(&span_contents)
                            {
                                item.sig.generics.params
                            } else {
                                panic!("Could not parse as a function or method:\n{span_contents}")
                            }
                        }
                        _ => unreachable!(),
                    };

                    let mut labels = vec![];
                    for param in generic_params {
                        if let syn::GenericParam::Type(ty) = param {
                            if free_parameters.contains(ty.ident.to_string().as_str()) {
                                labels.push(
                                    convert_proc_macro_span(&span_contents, ty.span()).labeled(
                                        "The generic parameter without a concrete type".into(),
                                    ),
                                );
                            }
                        }
                    }
                    let source_path = definition_span.filename.to_str().unwrap();
                    Some(AnnotatedSnippet::new_with_labels(
                        NamedSource::new(source_path, span_contents),
                        labels,
                    ))
                }

                let callable = &computation_db[user_component_id];
                let definition_snippet =
                    get_definition_span(callable, parameters, krate_collection, package_graph);
                let free_parameters = if parameters.len() == 1 {
                    format!("`{}`", &parameters[0])
                } else {
                    let mut buffer = String::new();
                    comma_separated_list(
                        &mut buffer,
                        parameters.iter(),
                        |p| format!("`{}`", p),
                        "and",
                    )
                    .unwrap();
                    buffer
                };
                let verb = if parameters.len() == 1 { "does" } else { "do" };
                let plural = if parameters.len() == 1 { "" } else { "s" };
                let error = anyhow::anyhow!(e)
                    .context(
                        format!(
                            "I am not smart enough to figure out the concrete type for all the generic parameters in `{}`.\n\
                            There should no unassigned generic parameters in request handlers, but {free_parameters} {verb} \
                            not seem to have been assigned a concrete type.",
                            callable.path));
                CompilerDiagnostic::builder(source, error)
                    .optional_label(label)
                    .optional_additional_annotated_snippet(definition_snippet)
                    .help(
                        format!("Specify the concrete type{plural} for {free_parameters} when registering the request handler against the blueprint: \n\
                        |  bp.route(\n\
                        |    ..\n\
                        |    f!(my_crate::my_handler::<ConcreteType>), \n\
                        |  )"))
                    // ^ TODO: add a proper code snippet here, using the actual function that needs
                    //    to be amended instead of a made signature
                    .build()
            }
        };
        diagnostics.push(diagnostic.into());
    }

    pub(super) fn invalid_wrapping_middleware(
        e: WrappingMiddlewareValidationError,
        user_component_id: UserComponentId,
        user_component_db: &UserComponentDb,
        computation_db: &ComputationDb,
        krate_collection: &CrateCollection,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        use crate::compiler::component::WrappingMiddlewareValidationError::*;

        let location = user_component_db.get_location(user_component_id);
        let source = source_or_exit_with_error!(location, package_graph, diagnostics);
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .labeled("The wrapping middleware was registered here".into());
        let diagnostic = match e {
            CannotReturnTheUnitType
            | CannotFalliblyReturnTheUnitType
            | MustTakeNextAsInputParameter => CompilerDiagnostic::builder(source, e)
                .optional_label(label)
                .build(),
            CannotTakeMoreThanOneNextAsInputParameter => CompilerDiagnostic::builder(source, e)
                .optional_label(label)
                .help("Remove the extra `Next` input parameters until only one is left.".into())
                .build(),
            NextGenericParameterMustBeNaked { ref parameter } => {
                let help =
                    format!("Take `Next<T>` rather than `Next<{parameter}>` as input parameter in your middleware.");
                CompilerDiagnostic::builder(source, e)
                    .optional_label(label)
                    .help(help)
                    .build()
            }
            UnderconstrainedGenericParameters { ref parameters } => {
                fn get_snippet(
                    callable: &Callable,
                    free_parameters: &IndexSet<String>,
                    krate_collection: &CrateCollection,
                    package_graph: &PackageGraph,
                ) -> Option<AnnotatedSnippet> {
                    let def =
                        CallableDefinition::compute(callable, krate_collection, package_graph)?;

                    let mut labels = vec![];
                    for param in &def.sig.generics.params {
                        if let syn::GenericParam::Type(ty) = param {
                            if free_parameters.contains(ty.ident.to_string().as_str()) {
                                labels.push(
                                    convert_proc_macro_span(&def.span_contents, ty.span()).labeled(
                                        "The generic parameter without a concrete type".into(),
                                    ),
                                );
                            }
                        }
                    }
                    Some(AnnotatedSnippet::new_with_labels(
                        def.named_source(),
                        labels,
                    ))
                }

                let callable = &computation_db[user_component_id];
                let definition_snippet =
                    get_snippet(callable, parameters, krate_collection, package_graph);
                let free_parameters = if parameters.len() == 1 {
                    format!("`{}`", &parameters[0])
                } else {
                    let mut buffer = String::new();
                    comma_separated_list(
                        &mut buffer,
                        parameters.iter(),
                        |p| format!("`{}`", p),
                        "and",
                    )
                    .unwrap();
                    buffer
                };
                let verb = if parameters.len() == 1 { "does" } else { "do" };
                let plural = if parameters.len() == 1 { "" } else { "s" };
                let error = anyhow::anyhow!(e)
                    .context(
                        format!(
                            "I am not smart enough to figure out the concrete type for all the generic parameters in `{}`.\n\
                            There should no unassigned generic parameters in wrapping middlewares apart from the one in `Next<_>`, but {free_parameters} {verb} \
                            not seem to have been assigned a concrete type.",
                            callable.path));
                CompilerDiagnostic::builder(source, error)
                    .optional_label(label)
                    .optional_additional_annotated_snippet(definition_snippet)
                    .help(
                        format!("Specify the concrete type{plural} for {free_parameters} when registering the wrapping middleware against the blueprint: \n\
                        |  bp.wrap(\n\
                        |    f!(my_crate::my_middleware::<ConcreteType>), \n\
                        |  )"))
                    // ^ TODO: add a proper code snippet here, using the actual function that needs
                    //    to be amended instead of a made signature
                    .build()
            }
        };
        diagnostics.push(diagnostic.into());
    }

    pub(super) fn invalid_response_type(
        e: MissingTraitImplementationError,
        output_type: &ResolvedType,
        user_component_id: UserComponentId,
        user_component_db: &UserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = user_component_db.get_location(user_component_id);
        let callable_type = user_component_db[user_component_id].callable_type();
        let source = source_or_exit_with_error!(location, package_graph, diagnostics);
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .labeled(format!("The {callable_type} was registered here"));
        let error = anyhow::Error::from(e).context(format!(
            "I can't use the type returned by this {callable_type} to create an HTTP \
                response.\n\
                It doesn't implement `pavex::response::IntoResponse`."
        ));
        let help = format!("Implement `pavex::response::IntoResponse` for `{output_type:?}`.");
        let diagnostic = CompilerDiagnostic::builder(source, error)
            .optional_label(label)
            .help(help)
            .build();
        diagnostics.push(diagnostic.into());
    }

    pub(super) fn cannot_handle_into_response_implementation(
        e: CallableResolutionError,
        output_type: &ResolvedType,
        raw_user_component_id: UserComponentId,
        raw_user_component_db: &UserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = raw_user_component_db.get_location(raw_user_component_id);
        let callable_type = raw_user_component_db[raw_user_component_id].callable_type();
        let source = source_or_exit_with_error!(location, package_graph, diagnostics);
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .labeled(format!("The {callable_type} was registered here"));
        let error = anyhow::Error::from(e).context(format!(
            "Something went wrong when I tried to analyze the implementation of \
                `pavex::response::IntoResponse` for {output_type:?}, the type returned by \
                one of your {callable_type}s.\n\
                This is definitely a bug, I am sorry! Please file an issue on \
                https://github.com/LukeMathWalker/pavex"
        ));
        let diagnostic = CompilerDiagnostic::builder(source, error)
            .optional_label(label)
            .build();
        diagnostics.push(diagnostic.into());
    }

    pub(super) fn invalid_error_observer(
        e: ErrorObserverValidationError,
        raw_user_component_id: UserComponentId,
        raw_user_component_db: &UserComponentDb,
        computation_db: &ComputationDb,
        krate_collection: &CrateCollection,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = raw_user_component_db.get_location(raw_user_component_id);
        let source = source_or_exit_with_error!(location, package_graph, diagnostics);
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .labeled("The error observer was registered here".into());
        let diagnostic = match &e {
            // TODO: Add a sub-diagnostic showing the error handler signature, highlighting with
            //  a label the non-unit return type.
            ErrorObserverValidationError::MustReturnUnitType { .. } |
            // TODO: Add a sub-diagnostic showing the error handler signature, highlighting with
            //  a label the input types. 
            ErrorObserverValidationError::DoesNotTakeErrorReferenceAsInput { .. } => {
                CompilerDiagnostic::builder(source, e)
                    .optional_label(label)
                    .build()
            }
            ErrorObserverValidationError::UnassignedGenericParameters { ref parameters, .. } => {
                fn get_snippet(
                    callable: &Callable,
                    free_parameters: &IndexSet<String>,
                    krate_collection: &CrateCollection,
                    package_graph: &PackageGraph,
                ) -> Option<AnnotatedSnippet> {
                    let def = CallableDefinition::compute(
                        callable,
                        krate_collection,
                        package_graph,
                    )?;

                    let mut labels = vec![];
                    for param in &def.sig.generics.params {
                        if let syn::GenericParam::Type(ty) = param {
                            if free_parameters.contains(ty.ident.to_string().as_str()) {
                                labels.push(
                                    convert_proc_macro_span(&def.span_contents, ty.span())
                                        .labeled("I can't infer this".into()),
                                );
                            }
                        }
                    }
                    Some(AnnotatedSnippet::new_with_labels(
                        def.named_source(),
                        labels,
                    ))
                }

                let callable = &computation_db[raw_user_component_id];
                let definition_snippet =
                    get_snippet(callable, parameters, krate_collection, package_graph);
                CompilerDiagnostic::builder(source, e)
                    .optional_label(label)
                    .optional_additional_annotated_snippet(definition_snippet)
                    .help(
                        "Specify the concrete type(s) for the problematic \
                        generic parameter(s) when registering the error observer against the blueprint: `f!(my_crate::my_observer::<ConcreteType>)`".into())
                    // ^ TODO: add a proper code snippet here, using the actual function that needs
                    //    to be amended instead of a made signature
                    .build()
            }
        };
        diagnostics.push(diagnostic.into());
    }

    pub(super) fn invalid_error_handler(
        e: ErrorHandlerValidationError,
        raw_user_component_id: UserComponentId,
        raw_user_component_db: &UserComponentDb,
        computation_db: &ComputationDb,
        krate_collection: &CrateCollection,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let location = raw_user_component_db.get_location(raw_user_component_id);
        let source = source_or_exit_with_error!(location, package_graph, diagnostics);
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .labeled("The error handler was registered here".into());
        let diagnostic = match &e {
            ErrorHandlerValidationError::CannotReturnTheUnitType(_) |
            // TODO: Perhaps add a snippet showing the signature of
            //  the associate fallible handler, highlighting the output type.
            ErrorHandlerValidationError::DoesNotTakeErrorReferenceAsInput { .. } => {
                CompilerDiagnostic::builder(source, e)
                    .optional_label(label)
                    .build()
            }
            ErrorHandlerValidationError::CannotBeFallible(_) => {
                fn get_snippet(
                    callable: &Callable,
                    krate_collection: &CrateCollection,
                    package_graph: &PackageGraph,
                ) -> Option<AnnotatedSnippet> {
                    let def = CallableDefinition::compute(
                        callable,
                        krate_collection,
                        package_graph,
                    )?;

                    let label = if let syn::ReturnType::Type(_, ty) = &def.sig.output {
                        Some(convert_proc_macro_span(&def.span_contents, ty.span())
                            .labeled("The output type".into()))
                    } else {
                        None
                    };
                    Some(AnnotatedSnippet::new_optional(
                        def.named_source(),
                        label,
                    ))
                }

                let definition_snippet =
                    get_snippet(&computation_db[raw_user_component_id], krate_collection, package_graph);
                CompilerDiagnostic::builder(source, e)
                    .optional_additional_annotated_snippet(definition_snippet)
                    .build()
            },
            ErrorHandlerValidationError::UnderconstrainedGenericParameters { ref parameters, ref error_ref_input_index } => {
                fn get_snippet(
                    callable: &Callable,
                    free_parameters: &IndexSet<String>,
                    error_ref_input_index: usize,
                    krate_collection: &CrateCollection,
                    package_graph: &PackageGraph,
                ) -> Option<AnnotatedSnippet> {
                    let callable_definition = CallableDefinition::compute(
                        callable,
                        krate_collection,
                        package_graph,
                    )?;
                    let error_input = callable_definition.sig.inputs[error_ref_input_index].clone();
                    let generic_params = &callable_definition.sig.generics.params;

                    let mut labels = vec![];
                    let subject_verb = if generic_params.len() == 1 {
                        "it is"
                    } else {
                        "they are"
                    };
                    for param in generic_params {
                        if let syn::GenericParam::Type(ty) = param {
                            if free_parameters.contains(ty.ident.to_string().as_str()) {
                                labels.push(
                                    convert_proc_macro_span(&callable_definition.span_contents, ty.span())
                                        .labeled("I can't infer this..".into()),
                                );
                            }
                        }
                    }
                    let error_input_span = error_input.span();
                    labels.push(
                        convert_proc_macro_span(&callable_definition.span_contents, error_input_span)
                            .labeled(format!("..because {subject_verb} not used here")),
                    );
                    Some(AnnotatedSnippet::new_with_labels(
                        callable_definition.named_source(),
                        labels,
                    ))
                }

                let callable = &computation_db[raw_user_component_id];
                let definition_snippet =
                    get_snippet(callable, parameters, *error_ref_input_index, krate_collection, package_graph);
                let subject_verb = if parameters.len() == 1 {
                    "it isn't"
                } else {
                    "they aren't"
                };
                let free_parameters = if parameters.len() == 1 {
                    format!("`{}`", &parameters[0])
                } else {
                    let mut buffer = String::new();
                    comma_separated_list(&mut buffer, parameters.iter(), |p| format!("`{}`", p), "and").unwrap();
                    buffer
                };
                let error = anyhow::anyhow!(e)
                    .context(
                        format!(
                            "I am not smart enough to figure out the concrete type for all the generic parameters in `{}`.\n\
                            I can only infer the type of an unassigned generic parameter if it appears in the error type processed by this error handler. This is \
                            not the case for {free_parameters}, since {subject_verb} used by the error type.",
                            callable.path));
                CompilerDiagnostic::builder(source, error)
                    .optional_label(label)
                    .optional_additional_annotated_snippet(definition_snippet)
                    .help(
                        "Specify the concrete type(s) for the problematic \
                        generic parameter(s) when registering the error handler against the blueprint: \n\
                        |  .error_handler(\n\
                        |    f!(my_crate::my_error_handler::<ConcreteType>)\n\
                        |  )".into())
                    // ^ TODO: add a proper code snippet here, using the actual function that needs
                    //    to be amended instead of a made signature
                    .build()
            }
        };
        diagnostics.push(diagnostic.into());
    }

    pub(super) fn error_handler_for_infallible_component(
        error_handler_id: UserComponentId,
        fallible_id: UserComponentId,
        raw_user_component_db: &UserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let fallible_kind = raw_user_component_db[fallible_id].callable_type();
        let location = raw_user_component_db.get_location(error_handler_id);
        let source = source_or_exit_with_error!(location, package_graph, diagnostics);
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .labeled("The unnecessary error handler was registered here".into());
        let error = anyhow::anyhow!(
            "You registered an error handler for a {} that doesn't return a `Result`.",
            fallible_kind
        );
        let diagnostic = CompilerDiagnostic::builder(source, error)
            .optional_label(label)
            .help(format!(
                "Remove the error handler, it is not needed. The {fallible_kind} is infallible!"
            ))
            .build();
        diagnostics.push(diagnostic.into());
    }

    pub(super) fn error_handler_for_a_singleton(
        error_handler_id: UserComponentId,
        fallible_id: UserComponentId,
        raw_user_component_db: &UserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        debug_assert_eq!(
            raw_user_component_db[fallible_id].callable_type(),
            CallableType::Constructor
        );
        let location = raw_user_component_db.get_location(error_handler_id);
        let source = source_or_exit_with_error!(location, package_graph, diagnostics);
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .labeled("The unnecessary error handler was registered here".into());
        let error = anyhow::anyhow!(
            "You can't register an error handler for a singleton constructor. \n\
                If I fail to build a singleton, I bubble up the error - it doesn't get handled.",
        );
        let diagnostic = CompilerDiagnostic::builder(source, error)
            .optional_label(label)
            .help("Remove the error handler, it is not needed.".to_string())
            .build();
        diagnostics.push(diagnostic.into());
    }

    pub(super) fn missing_error_handler(
        fallible_id: UserComponentId,
        raw_user_component_db: &UserComponentDb,
        package_graph: &PackageGraph,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        let fallible_kind = raw_user_component_db[fallible_id].callable_type();
        let location = raw_user_component_db.get_location(fallible_id);
        let source = source_or_exit_with_error!(location, package_graph, diagnostics);
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .labeled(format!("The fallible {fallible_kind} was registered here"));
        let error = anyhow::anyhow!(
                "You registered a {fallible_kind} that returns a `Result`, but you did not register an \
                 error handler for it. \
                 If I don't have an error handler, I don't know what to do with the error when the \
                 {fallible_kind} fails!",
            );
        let diagnostic = CompilerDiagnostic::builder(source, error)
            .optional_label(label)
            .help("Add an error handler via `.error_handler`".to_string())
            .build();
        diagnostics.push(diagnostic.into());
    }
}
