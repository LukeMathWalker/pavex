use crate::compiler::analyses::components::ComponentDb;
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::user_components::{UserComponentDb, UserComponentId};
use crate::compiler::component::{
    ConstructorValidationError, ErrorHandlerValidationError, ErrorObserverValidationError,
    PostProcessingMiddlewareValidationError, PreProcessingMiddlewareValidationError,
    RequestHandlerValidationError, WrappingMiddlewareValidationError,
};
use crate::compiler::resolvers::CallableResolutionError;
use crate::compiler::traits::MissingTraitImplementationError;
use crate::diagnostic::{
    self, AnnotatedSource, CallableDefSource, CompilerDiagnostic, ComponentKind, SourceSpanExt,
    convert_proc_macro_span, convert_rustdoc_span,
};
use crate::language::{Callable, ResolvedType};
use crate::rustdoc::CrateCollection;
use crate::utils::comma_separated_list;
use guppy::graph::PackageGraph;
use indexmap::IndexSet;
use miette::{NamedSource, Severity};
use rustdoc_types::ItemEnum;
use syn::spanned::Spanned;

use super::get_err_variant;

/// Utility functions to produce diagnostics.
impl ComponentDb {
    pub(super) fn invalid_constructor(
        e: ConstructorValidationError,
        id: UserComponentId,
        db: &UserComponentDb,
        computation_db: &ComputationDb,
        krate_collection: &CrateCollection,
        diagnostics: &diagnostic::DiagnosticSink,
    ) {
        let source = diagnostics.annotated(
            db.registration_target(id),
            "The constructor was registered here",
        );
        match e {
            ConstructorValidationError::CannotTakeAMutableReferenceAsInput(inner) => {
                inner.emit(
                    id,
                    db,
                    computation_db,
                    krate_collection,
                    ComponentKind::Constructor,
                    diagnostics,
                );
            }
            ConstructorValidationError::CannotFalliblyReturnTheUnitType
            | ConstructorValidationError::CannotConstructPavexError
            | ConstructorValidationError::CannotConstructPavexResponse
            | ConstructorValidationError::CannotConstructFrameworkPrimitive { .. }
            | ConstructorValidationError::CannotReturnTheUnitType => {
                let d = CompilerDiagnostic::builder(e)
                    .optional_source(source)
                    .build();
                diagnostics.push(d);
            }
            ConstructorValidationError::UnderconstrainedGenericParameters { ref parameters } => {
                fn get_definition_span(
                    callable: &Callable,
                    free_parameters: &IndexSet<String>,
                    krate_collection: &CrateCollection,
                ) -> Option<AnnotatedSource<NamedSource<String>>> {
                    let mut def = CallableDefSource::compute(callable, krate_collection)?;

                    let params = def.sig.generics.params.clone();
                    let subject_verb = if params.len() == 1 {
                        "it is"
                    } else {
                        "they are"
                    };
                    for param in &params {
                        if let syn::GenericParam::Type(ty) = param
                            && free_parameters.contains(ty.ident.to_string().as_str())
                        {
                            def.label(ty, "I can't infer this..");
                        }
                    }
                    def.label_output(format!("..because {subject_verb} not used here"));
                    Some(def.annotated_source)
                }

                let callable = &computation_db[id];
                let definition_snippet =
                    get_definition_span(callable, parameters, krate_collection);
                let subject_verb = if parameters.len() == 1 { "is" } else { "are" };
                let free_parameters = if parameters.len() == 1 {
                    format!("`{}`", &parameters[0])
                } else {
                    let mut buffer = String::new();
                    comma_separated_list(
                        &mut buffer,
                        parameters.iter(),
                        |p| format!("`{p}`"),
                        "and",
                    )
                    .unwrap();
                    buffer
                };
                let error = anyhow::anyhow!(e)
                    .context(
                        format!(
                            "All unassigned generic parameters must be used by the output type.\n\
                            `{}`, one of your constructors, breaks this rule: {free_parameters} {subject_verb} only used by its input parameters.",
                            callable.path));
                let help = if db.registration(id).kind.is_blueprint() {
                    Some("Assign concrete type(s) to the problematic \
                        generic parameter(s) when registering the constructor against the blueprint: \n\
                        |  bp.constructor(\n\
                        |    f!(my_crate::my_constructor::<ConcreteType>), \n\
                        |    ..\n\
                        |  )".to_string())
                } else {
                    None
                };
                let d = CompilerDiagnostic::builder(error)
                    .optional_source(source)
                    .optional_source(definition_snippet)
                    .help(
                        "Can you restructure your constructor to remove those generic parameters from its signature?"
                            .into(),
                    )
                    .optional_help(help)
                    // ^ TODO: add a proper code snippet here, using the actual function that needs
                    //    to be amended instead of a made signature
                    .build();
                diagnostics.push(d);
            }
            ConstructorValidationError::NakedGenericOutputType {
                ref naked_parameter,
            } => {
                fn get_definition_span(
                    callable: &Callable,
                    krate_collection: &CrateCollection,
                ) -> Option<AnnotatedSource<NamedSource<String>>> {
                    let global_item_id = callable.source_coordinates.as_ref()?;
                    let item = krate_collection.get_item_by_global_type_id(global_item_id);
                    let definition_span = item.span.as_ref()?;
                    let source_contents = diagnostic::read_source_file(
                        &definition_span.filename,
                        &krate_collection.package_graph().workspace(),
                    )
                    .ok()?;
                    let span = convert_rustdoc_span(&source_contents, definition_span.to_owned());
                    let span_contents =
                        source_contents[span.offset()..(span.offset() + span.len())].to_string();
                    let output = match &item.inner {
                        ItemEnum::Function(_) => {
                            match syn::parse_str::<syn::ItemFn>(&span_contents) {
                                Ok(item) => item.sig.output,
                                _ => match syn::parse_str::<syn::ImplItemFn>(&span_contents) {
                                    Ok(item) => item.sig.output,
                                    _ => match syn::parse_str::<syn::TraitItemFn>(&span_contents) {
                                        Ok(item) => item.sig.output,
                                        _ => {
                                            panic!(
                                                "Could not parse as a function or method:\n{span_contents}"
                                            )
                                        }
                                    },
                                },
                            }
                        }
                        _ => unreachable!(),
                    };

                    let output_span = match &output {
                        syn::ReturnType::Type(_, output_type) => output_type.span(),
                        _ => output.span(),
                    };
                    let label = convert_proc_macro_span(&span_contents, output_span)
                        .labeled("The invalid output type".to_string());
                    let source_path = definition_span.filename.to_str().unwrap();
                    Some(
                        AnnotatedSource::new(NamedSource::new(source_path, span_contents))
                            .label(label),
                    )
                }

                let callable = &computation_db[id];
                let definition_snippet = get_definition_span(callable, krate_collection);
                let msg = format!(
                    "You can't return a naked generic parameter from a constructor, like `{naked_parameter}` in `{}`.\n\
                    I don't take into account trait bounds when building your dependency graph. A constructor \
                    that returns a naked generic parameter is equivalent, in my eyes, to a constructor that can build \
                    **any** type, which is unlikely to be what you want!",
                    callable.path
                );
                let error = anyhow::anyhow!(e).context(msg);
                let d = CompilerDiagnostic::builder(error)
                    .optional_source(source)
                    .optional_source(definition_snippet)
                    .help(
                        "Can you return a concrete type as output? \n\
                        Or wrap the generic parameter in a non-generic container? \
                        For example, `T` in `Vec<T>` is not considered to be a naked parameter."
                            .into(),
                    )
                    .build();
                diagnostics.push(d);
            }
        };
    }

    pub(super) fn invalid_request_handler(
        e: RequestHandlerValidationError,
        id: UserComponentId,
        db: &UserComponentDb,
        computation_db: &ComputationDb,
        krate_collection: &CrateCollection,
        package_graph: &PackageGraph,
        diagnostics: &diagnostic::DiagnosticSink,
    ) {
        let source = diagnostics.annotated(
            db.registration_target(id),
            "The request handler was registered here",
        );
        match e {
            RequestHandlerValidationError::CannotTakeAMutableReferenceAsInput(inner) => {
                inner.emit(
                    id,
                    db,
                    computation_db,
                    krate_collection,
                    ComponentKind::RequestHandler,
                    diagnostics,
                );
            }
            RequestHandlerValidationError::CannotReturnTheUnitType
            | RequestHandlerValidationError::CannotFalliblyReturnTheUnitType => {
                let d = CompilerDiagnostic::builder(e)
                    .optional_source(source)
                    .build();
                diagnostics.push(d);
            }
            RequestHandlerValidationError::UnderconstrainedGenericParameters { ref parameters } => {
                fn get_definition_span(
                    callable: &Callable,
                    free_parameters: &IndexSet<String>,
                    krate_collection: &CrateCollection,
                    package_graph: &PackageGraph,
                ) -> Option<AnnotatedSource<NamedSource<String>>> {
                    let global_item_id = callable.source_coordinates.as_ref()?;
                    let item = krate_collection.get_item_by_global_type_id(global_item_id);
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
                            match syn::parse_str::<syn::ItemFn>(&span_contents) {
                                Ok(item) => item.sig.generics.params,
                                _ => match syn::parse_str::<syn::ImplItemFn>(&span_contents) {
                                    Ok(item) => item.sig.generics.params,
                                    _ => match syn::parse_str::<syn::TraitItemFn>(&span_contents) {
                                        Ok(item) => item.sig.generics.params,
                                        _ => {
                                            panic!(
                                                "Could not parse as a function or method:\n{span_contents}"
                                            )
                                        }
                                    },
                                },
                            }
                        }
                        _ => unreachable!(),
                    };

                    let mut labels = vec![];
                    for param in generic_params {
                        if let syn::GenericParam::Type(ty) = param
                            && free_parameters.contains(ty.ident.to_string().as_str())
                        {
                            labels.push(
                                convert_proc_macro_span(&span_contents, ty.span()).labeled(
                                    "The generic parameter without a concrete type".into(),
                                ),
                            );
                        }
                    }
                    let source_path = definition_span.filename.to_str().unwrap();
                    Some(
                        AnnotatedSource::new(NamedSource::new(source_path, span_contents))
                            .labels(labels),
                    )
                }

                let callable = &computation_db[id];
                let definition_snippet =
                    get_definition_span(callable, parameters, krate_collection, package_graph);
                let free_parameters = if parameters.len() == 1 {
                    format!("`{}`", &parameters[0])
                } else {
                    let mut buffer = String::new();
                    comma_separated_list(
                        &mut buffer,
                        parameters.iter(),
                        |p| format!("`{p}`"),
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
                let d = CompilerDiagnostic::builder(error).optional_source(source)
                    .optional_source(definition_snippet)
                    .help(
                        format!("Specify the concrete type{plural} for {free_parameters} when registering the request handler against the blueprint: \n\
                        |  bp.route(\n\
                        |    ..\n\
                        |    f!(my_crate::my_handler::<ConcreteType>), \n\
                        |  )"))
                    // ^ TODO: add a proper code snippet here, using the actual function that needs
                    //    to be amended instead of a made signature
                    .build();
                diagnostics.push(d);
            }
        };
    }

    pub(super) fn invalid_wrapping_middleware(
        e: WrappingMiddlewareValidationError,
        id: UserComponentId,
        db: &UserComponentDb,
        computation_db: &ComputationDb,
        krate_collection: &CrateCollection,
        diagnostics: &diagnostic::DiagnosticSink,
    ) {
        use crate::compiler::component::WrappingMiddlewareValidationError::*;

        let source = diagnostics.annotated(
            db.registration_target(id),
            "The wrapping middleware was registered here",
        );
        match e {
            CannotTakeAMutableReferenceAsInput(inner) => {
                inner.emit(
                    id,
                    db,
                    computation_db,
                    krate_collection,
                    ComponentKind::WrappingMiddleware,
                    diagnostics,
                );
            }
            CannotReturnTheUnitType
            | CannotFalliblyReturnTheUnitType
            | MustTakeNextAsInputParameter => {
                let d = CompilerDiagnostic::builder(e)
                    .optional_source(source)
                    .build();
                diagnostics.push(d);
            }
            CannotTakeMoreThanOneNextAsInputParameter => {
                let d = CompilerDiagnostic::builder(e)
                    .optional_source(source)
                    .help("Remove the extra `Next` input parameters until only one is left.".into())
                    .build();
                diagnostics.push(d);
            }
            NextGenericParameterMustBeNaked { ref parameter } => {
                let help = format!(
                    "Take `Next<T>` rather than `Next<{parameter}>` as input parameter in your middleware."
                );
                let d = CompilerDiagnostic::builder(e)
                    .optional_source(source)
                    .help(help)
                    .build();
                diagnostics.push(d);
            }
            UnderconstrainedGenericParameters { ref parameters } => {
                fn get_snippet(
                    callable: &Callable,
                    free_parameters: &IndexSet<String>,
                    krate_collection: &CrateCollection,
                ) -> Option<AnnotatedSource<NamedSource<String>>> {
                    let mut def = CallableDefSource::compute(callable, krate_collection)?;

                    for param in def.sig.generics.params.clone() {
                        if let syn::GenericParam::Type(ty) = param
                            && free_parameters.contains(ty.ident.to_string().as_str())
                        {
                            def.label(ty, "The generic parameter without a concrete type");
                        }
                    }
                    Some(def.annotated_source)
                }

                let callable = &computation_db[id];
                let definition_snippet = get_snippet(callable, parameters, krate_collection);
                let free_parameters = if parameters.len() == 1 {
                    format!("`{}`", &parameters[0])
                } else {
                    let mut buffer = String::new();
                    comma_separated_list(
                        &mut buffer,
                        parameters.iter(),
                        |p| format!("`{p}`"),
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
                let d = CompilerDiagnostic::builder(error)
                    .optional_source(source)
                    .optional_source(definition_snippet)
                    .help(
                        format!("Specify the concrete type{plural} for {free_parameters} when registering the wrapping middleware against the blueprint: \n\
                        |  bp.wrap(\n\
                        |    f!(my_crate::my_middleware::<ConcreteType>), \n\
                        |  )"))
                    // ^ TODO: add a proper code snippet here, using the actual function that needs
                    //    to be amended instead of a made signature
                    .build();
                diagnostics.push(d);
            }
        };
    }

    pub(super) fn invalid_pre_processing_middleware(
        e: PreProcessingMiddlewareValidationError,
        id: UserComponentId,
        db: &UserComponentDb,
        computation_db: &ComputationDb,
        krate_collection: &CrateCollection,
        diagnostics: &diagnostic::DiagnosticSink,
    ) {
        use crate::compiler::component::PreProcessingMiddlewareValidationError::*;

        let source = diagnostics.annotated(
            db.registration_target(id),
            "The pre-processing middleware was registered here",
        );
        match e {
            CannotReturnTheUnitType | CannotFalliblyReturnTheUnitType => {
                let d = CompilerDiagnostic::builder(e)
                    .optional_source(source)
                    .build();
                diagnostics.push(d);
            }
            UnderconstrainedGenericParameters { ref parameters } => {
                fn get_snippet(
                    callable: &Callable,
                    free_parameters: &IndexSet<String>,
                    krate_collection: &CrateCollection,
                ) -> Option<AnnotatedSource<NamedSource<String>>> {
                    let mut def = CallableDefSource::compute(callable, krate_collection)?;
                    for param in def.sig.generics.params.clone() {
                        if let syn::GenericParam::Type(ty) = param
                            && free_parameters.contains(ty.ident.to_string().as_str())
                        {
                            def.label(ty, "The generic parameter without a concrete type");
                        }
                    }
                    Some(def.annotated_source)
                }

                let callable = &computation_db[id];
                let definition_snippet = get_snippet(callable, parameters, krate_collection);
                let free_parameters = if parameters.len() == 1 {
                    format!("`{}`", &parameters[0])
                } else {
                    let mut buffer = String::new();
                    comma_separated_list(
                        &mut buffer,
                        parameters.iter(),
                        |p| format!("`{p}`"),
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
                            "There must be no unassigned generic parameters in pre-processing middlewares, but {free_parameters} {verb} \
                            not seem to have been assigned a concrete type in `{}`.",
                            callable.path));
                let d = CompilerDiagnostic::builder(error)
                    .optional_source(source)
                    .optional_source(definition_snippet)
                    .help(
                        format!("Specify the concrete type{plural} for {free_parameters} when registering the pre-processing middleware against the blueprint: \n\
                        |  bp.pre_process(\n\
                        |    f!(my_crate::my_middleware::<ConcreteType>), \n\
                        |  )"))
                    // ^ TODO: add a proper code snippet here, using the actual function that needs
                    //    to be amended instead of a made signature
                    .build();
                diagnostics.push(d);
            }
        };
    }

    pub(super) fn invalid_post_processing_middleware(
        e: PostProcessingMiddlewareValidationError,
        id: UserComponentId,
        db: &UserComponentDb,
        computation_db: &ComputationDb,
        krate_collection: &CrateCollection,
        diagnostics: &diagnostic::DiagnosticSink,
    ) {
        use crate::compiler::component::PostProcessingMiddlewareValidationError::*;

        let source = diagnostics.annotated(
            db.registration_target(id),
            "The post-processing middleware was registered here",
        );
        match e {
            CannotReturnTheUnitType
            | CannotFalliblyReturnTheUnitType
            | MustTakeResponseAsInputParameter => {
                let d = CompilerDiagnostic::builder(e)
                    .optional_source(source)
                    .build();
                diagnostics.push(d);
            }
            CannotTakeMoreThanOneResponseAsInputParameter => {
                let d = CompilerDiagnostic::builder(e)
                    .optional_source(source)
                    .help(
                        "Remove the extra `Response` input parameters until only one is left."
                            .into(),
                    )
                    .build();
                diagnostics.push(d);
            }
            UnderconstrainedGenericParameters { ref parameters } => {
                fn get_snippet(
                    callable: &Callable,
                    free_parameters: &IndexSet<String>,
                    krate_collection: &CrateCollection,
                ) -> Option<AnnotatedSource<NamedSource<String>>> {
                    let mut def = CallableDefSource::compute(callable, krate_collection)?;
                    for param in def.sig.generics.params.clone() {
                        if let syn::GenericParam::Type(ty) = param
                            && free_parameters.contains(ty.ident.to_string().as_str())
                        {
                            def.label(ty, "The generic parameter without a concrete type");
                        }
                    }
                    Some(def.annotated_source)
                }

                let callable = &computation_db[id];
                let definition_snippet = get_snippet(callable, parameters, krate_collection);
                let free_parameters = if parameters.len() == 1 {
                    format!("`{}`", &parameters[0])
                } else {
                    let mut buffer = String::new();
                    comma_separated_list(
                        &mut buffer,
                        parameters.iter(),
                        |p| format!("`{p}`"),
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
                            There should no unassigned generic parameters in post-processing middlewares, but {free_parameters} {verb} \
                            not seem to have been assigned a concrete type.",
                            callable.path));
                let d = CompilerDiagnostic::builder(error)
                    .optional_source(source)
                    .optional_source(definition_snippet)
                    .help(
                        format!("Specify the concrete type{plural} for {free_parameters} when registering the post-processing middleware against the blueprint: \n\
                        |  bp.post_process(\n\
                        |    f!(my_crate::my_middleware::<ConcreteType>), \n\
                        |  )"))
                    // ^ TODO: add a proper code snippet here, using the actual function that needs
                    //    to be amended instead of a made signature
                    .build();
                diagnostics.push(d);
            }
        };
    }

    pub(super) fn invalid_response_type(
        e: MissingTraitImplementationError,
        callable: &Callable,
        output: &ResolvedType,
        id: UserComponentId,
        db: &UserComponentDb,
        krate_collection: &CrateCollection,
        diagnostics: &diagnostic::DiagnosticSink,
    ) {
        let kind = db[id].kind();
        let source = diagnostics.annotated(
            db.registration_target(id),
            format!("The {kind} was registered here"),
        );
        let path = &callable.path;
        let output = output.display_for_error();
        let error = anyhow::Error::from(e).context(format!(
            "`{output}` doesn't implement `pavex::IntoResponse`.\n\
            It is returned by `{path}`, one of your {kind}s.\n\
            `IntoResponse` is used by Pavex to convert `{output}` into the HTTP response that will be returned to the caller of your API.",
        ));
        let def_source = CallableDefSource::compute(callable, krate_collection).map(|mut def| {
            def.label_output("The faulty output type");
            def.annotated_source
        });
        let help = format!("Implement `IntoResponse` for `{output}`.");
        let diagnostic = CompilerDiagnostic::builder(error)
            .optional_source(source)
            .optional_source(def_source)
            .help(help)
            .build();
        diagnostics.push(diagnostic);
    }

    pub(super) fn cannot_handle_into_response_implementation(
        e: CallableResolutionError,
        output_type: &ResolvedType,
        id: UserComponentId,
        db: &UserComponentDb,
        diagnostics: &diagnostic::DiagnosticSink,
    ) {
        let kind = db[id].kind();
        let source = diagnostics.annotated(
            db.registration_target(id),
            format!("The {kind} was registered here"),
        );
        let error = anyhow::Error::from(e).context(format!(
            "Something went wrong when I tried to analyze the implementation of \
                `pavex::IntoResponse` for {output_type:?}, the type returned by \
                one of your {kind}s.\n\
                This is definitely a bug, I am sorry! Please file an issue on \
                https://github.com/LukeMathWalker/pavex"
        ));
        let diagnostic = CompilerDiagnostic::builder(error)
            .optional_source(source)
            .build();
        diagnostics.push(diagnostic);
    }

    pub(super) fn invalid_error_observer(
        e: ErrorObserverValidationError,
        id: UserComponentId,
        db: &UserComponentDb,
        computation_db: &ComputationDb,
        krate_collection: &CrateCollection,
        diagnostics: &diagnostic::DiagnosticSink,
    ) {
        let source = diagnostics.annotated(
            db.registration_target(id),
            "The error observer was registered here",
        );
        match e {
            ErrorObserverValidationError::CannotTakeAMutableReferenceAsInput(inner) => {
                inner.emit(id, db, computation_db, krate_collection, ComponentKind::ErrorObserver, diagnostics);
            }
            // TODO: Add a sub-diagnostic showing the error handler signature, highlighting with
            //  a label the non-unit return type.
            ErrorObserverValidationError::MustReturnUnitType { .. } |
            // TODO: Add a sub-diagnostic showing the error handler signature, highlighting with
            //  a label the input types.
            ErrorObserverValidationError::DoesNotTakeErrorReferenceAsInput { .. } => {
                let d = CompilerDiagnostic::builder(e).optional_source(source)
                    .build();
                diagnostics.push(d);
            }
            ErrorObserverValidationError::UnassignedGenericParameters { ref parameters, .. } => {
                fn get_snippet(
                    callable: &Callable,
                    free_parameters: &IndexSet<String>,
                    krate_collection: &CrateCollection,
                ) -> Option<AnnotatedSource<NamedSource<String>>> {
                    let mut def = CallableDefSource::compute(
                        callable,
                        krate_collection,
                    )?;
                    for param in def.sig.generics.params.clone() {
                        if let syn::GenericParam::Type(ty) = param
                            && free_parameters.contains(ty.ident.to_string().as_str()) {
                                def.label(ty, "I can't infer this");
                            }
                    }
                    Some(def.annotated_source)
                }

                let callable = &computation_db[id];
                let definition_snippet =
                    get_snippet(callable, parameters, krate_collection);
                let d = CompilerDiagnostic::builder(e).optional_source(source)
                    .optional_source(definition_snippet)
                    .help(
                        "Specify the concrete type(s) for the problematic \
                        generic parameter(s) when registering the error observer against the blueprint: `f!(my_crate::my_observer::<ConcreteType>)`".into())
                    // ^ TODO: add a proper code snippet here, using the actual function that needs
                    //    to be amended instead of a made signature
                    .build();
                diagnostics.push(d);
            },
        };
    }

    pub(super) fn invalid_error_handler(
        e: ErrorHandlerValidationError,
        id: UserComponentId,
        db: &UserComponentDb,
        computation_db: &ComputationDb,
        krate_collection: &CrateCollection,
        diagnostics: &diagnostic::DiagnosticSink,
    ) {
        let source = diagnostics.annotated(
            db.registration_target(id),
            "The error handler was registered here",
        );
        match e {
            ErrorHandlerValidationError::CannotReturnTheUnitType(_) |
            // TODO: Perhaps add a snippet showing the signature of
            //  the associate fallible handler, highlighting the output type.
            ErrorHandlerValidationError::DoesNotTakeErrorReferenceAsInput { .. } => {
                let diagnostic = CompilerDiagnostic::builder(e).optional_source(source)
                    .build();
                diagnostics.push(diagnostic);
            }
            ErrorHandlerValidationError::CannotBeFallible(_) => {
                fn get_snippet(
                    callable: &Callable,
                    krate_collection: &CrateCollection,
                ) -> Option<AnnotatedSource<NamedSource<String>>> {
                    let mut def = CallableDefSource::compute(
                        callable,
                        krate_collection,
                    )?;
                    def.label_output("The output type");
                    Some(def.annotated_source)
                }

                let definition_snippet =
                    get_snippet(&computation_db[id], krate_collection);
                let diagnostic = CompilerDiagnostic::builder(e).optional_source(source)
                    .optional_source(definition_snippet)
                    .build();
                diagnostics.push(diagnostic);
            },
            ErrorHandlerValidationError::UnderconstrainedGenericParameters { ref parameters, ref error_ref_input_index } => {
                fn get_snippet(
                    callable: &Callable,
                    free_parameters: &IndexSet<String>,
                    error_ref_input_index: usize,
                    krate_collection: &CrateCollection,
                ) -> Option<AnnotatedSource<NamedSource<String>>> {
                    let mut def = CallableDefSource::compute(
                        callable,
                        krate_collection,
                    )?;
                    let params = def.sig.generics.params.clone();
                    let subject_verb = if params.len() == 1 {
                        "it is"
                    } else {
                        "they are"
                    };
                    for param in params {
                        if let syn::GenericParam::Type(ty) = param
                            && free_parameters.contains(ty.ident.to_string().as_str()) {
                                def.label(ty, "I can't infer this..");
                            }
                    }
                    let error_input = def.sig.inputs[error_ref_input_index].clone();
                    def.label(error_input, format!("..because {subject_verb} not used here"));
                    Some(def.annotated_source)
                }

                let callable = &computation_db[id];
                let definition_snippet =
                    get_snippet(callable, parameters, *error_ref_input_index, krate_collection);
                let subject_verb = if parameters.len() == 1 {
                    "it isn't"
                } else {
                    "they aren't"
                };
                let free_parameters = if parameters.len() == 1 {
                    format!("`{}`", &parameters[0])
                } else {
                    let mut buffer = String::new();
                    comma_separated_list(&mut buffer, parameters.iter(), |p| format!("`{p}`"), "and").unwrap();
                    buffer
                };
                let error = anyhow::anyhow!(e)
                    .context(
                        format!(
                            "I am not smart enough to figure out the concrete type for all the generic parameters in `{}`.\n\
                            I can only infer the type of an unassigned generic parameter if it appears in the error type processed by this error handler. This is \
                            not the case for {free_parameters}, since {subject_verb} used by the error type.",
                            callable.path));
                let diagnostic = CompilerDiagnostic::builder(error).optional_source(source)
                    .optional_source(definition_snippet)
                    .help(
                        "Specify the concrete type(s) for the problematic \
                        generic parameter(s) when registering the error handler against the blueprint: \n\
                        |  .error_handler(\n\
                        |    f!(my_crate::my_error_handler::<ConcreteType>)\n\
                        |  )".into())
                    // ^ TODO: add a proper code snippet here, using the actual function that needs
                    //    to be amended instead of a made signature
                    .build();
                diagnostics.push(diagnostic);
            }
            ErrorHandlerValidationError::CannotTakeAMutableReferenceAsInput(inner) => {
                inner.emit(id, db, computation_db, krate_collection, ComponentKind::ErrorHandler, diagnostics);
            }
        };
    }

    pub(super) fn error_handler_for_infallible_component(
        error_handler_id: UserComponentId,
        fallible_id: UserComponentId,
        db: &UserComponentDb,
        diagnostics: &diagnostic::DiagnosticSink,
    ) {
        let fallible_kind = db[fallible_id].kind();
        let source = diagnostics.annotated(
            db.registration_target(error_handler_id),
            "The unnecessary error handler was registered here",
        );
        let error = anyhow::anyhow!(
            "You registered an error handler for a {} that doesn't return a `Result`.",
            fallible_kind
        );
        let diagnostic = CompilerDiagnostic::builder(error)
            .optional_source(source)
            .help(format!(
                "Remove the error handler, it is not needed. The {fallible_kind} is infallible!"
            ))
            .build();
        diagnostics.push(diagnostic);
    }

    pub(super) fn error_handler_for_a_singleton(
        error_handler_id: UserComponentId,
        fallible_id: UserComponentId,
        db: &UserComponentDb,
        diagnostics: &diagnostic::DiagnosticSink,
    ) {
        debug_assert_eq!(db[fallible_id].kind(), ComponentKind::Constructor);
        let source = diagnostics.annotated(
            db.registration_target(error_handler_id),
            "The unnecessary error handler was registered here",
        );
        let error = anyhow::anyhow!(
            "You can't register an error handler for a singleton constructor. \n\
                If I fail to build a singleton, I bubble up the error - it doesn't get handled.",
        );
        let diagnostic = CompilerDiagnostic::builder(error)
            .optional_source(source)
            .help("Remove the error handler, it is not needed.".to_string())
            .build();
        diagnostics.push(diagnostic);
    }

    pub(super) fn missing_error_handler(
        fallible_id: UserComponentId,
        db: &UserComponentDb,
        computation_db: &ComputationDb,
        diagnostics: &diagnostic::DiagnosticSink,
    ) {
        let fallible_kind = db[fallible_id].kind();
        let source = diagnostics.annotated(
            db.registration_target(fallible_id),
            format!("The fallible {fallible_kind} was registered here"),
        );
        let fallible = &computation_db[fallible_id];
        let err_ty = get_err_variant(fallible.output.as_ref().unwrap());
        let error = anyhow::anyhow!(
            "There is no specific error handler for `{}`, the error returned by one of your {fallible_kind}s.\n\
            It'll be converted to `pavex::Error` and handled by the fallback error handler.",
            err_ty.display_for_error()
        );
        let diagnostic = CompilerDiagnostic::builder(error)
            .severity(Severity::Warning)
            .optional_source(source)
            .help(format!(
                "Define an error handler for `{}`",
                err_ty.display_for_error()
            ))
            .help(
                "Add `allow(error_fallback)` to your component's attribute to silence this warning"
                    .into(),
            )
            .build();
        diagnostics.push(diagnostic);
    }

    pub(super) fn non_static_reference_in_singleton(
        output_type: &ResolvedType,
        id: UserComponentId,
        db: &UserComponentDb,
        diagnostics: &diagnostic::DiagnosticSink,
    ) {
        let source = diagnostics.annotated(
            db.registration_target(id),
            "The singleton was registered here",
        );
        let error = anyhow::anyhow!(
            "`{output_type:?}` can't be a singleton because its lifetime isn't `'static`.\n\
            Singletons must be available for as long as the application is running, \
            therefore their lifetime must be `'static`.",
        );
        let d = CompilerDiagnostic::builder(error)
            .optional_source(source)
            .help(
                "If you are returning a reference to data that's owned by another singleton component, \
                register the constructor as transient rather than singleton.".into(),
            )
            .build();
        diagnostics.push(d);
    }

    pub(super) fn non_static_lifetime_parameter_in_singleton(
        output_type: &ResolvedType,
        id: UserComponentId,
        db: &UserComponentDb,
        diagnostics: &diagnostic::DiagnosticSink,
    ) {
        let source = diagnostics.annotated(
            db.registration_target(id),
            "The singleton was registered here",
        );
        let error = anyhow::anyhow!(
            "`{output_type:?}` can't be a singleton because at least one of its lifetime parameters isn't `'static`.\n\
            Singletons must be available for as long as the application is running, \
            therefore their lifetime must be `'static`.",
        );
        let d = CompilerDiagnostic::builder(error)
            .optional_source(source)
            .help(
                "If your type holds a reference to data that's owned by another singleton component, \
                register its constructor as transient rather than singleton.".into(),
            )
            .build();
        diagnostics.push(d);
    }
}
