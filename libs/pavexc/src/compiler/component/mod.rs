mod config_type;
mod constructor;
mod error_handler;
mod error_observer;
mod post_processing_middleware;
mod pre_processing_middleware;
mod prebuilt_type;
mod request_handler;
mod wrapping_middleware;

use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::user_components::{UserComponentDb, UserComponentId};
use crate::diagnostic::{
    self, AnnotatedSource, CallableDefinition, CompilerDiagnostic, ComponentKind,
    OptionalLabeledSpanExt, OptionalSourceSpanExt, SourceSpanExt,
};
use crate::language::{Callable, ResolvedPath, ResolvedType};
use crate::rustdoc::CrateCollection;
use guppy::graph::PackageGraph;
use miette::NamedSource;
use syn::spanned::Spanned;

pub(crate) use config_type::{ConfigKey, ConfigType, ConfigTypeValidationError, DefaultStrategy};
pub(crate) use constructor::{Constructor, ConstructorValidationError};
pub(crate) use error_handler::{ErrorHandler, ErrorHandlerValidationError};
pub(crate) use error_observer::{ErrorObserver, ErrorObserverValidationError};
pub(crate) use post_processing_middleware::{
    PostProcessingMiddleware, PostProcessingMiddlewareValidationError,
};
pub(crate) use pre_processing_middleware::{
    PreProcessingMiddleware, PreProcessingMiddlewareValidationError,
};
pub(crate) use prebuilt_type::{PrebuiltType, PrebuiltTypeValidationError};
pub(crate) use request_handler::{RequestHandler, RequestHandlerValidationError};
pub(crate) use wrapping_middleware::{WrappingMiddleware, WrappingMiddlewareValidationError};

#[derive(thiserror::Error, Debug, Clone)]
#[error("You can't inject a mutable reference as an input parameter to `{component_path}`.")]
pub(crate) struct CannotTakeMutReferenceError {
    pub component_path: ResolvedPath,
    pub mut_ref_input_index: usize,
}

impl CannotTakeMutReferenceError {
    pub(crate) fn check_callable(c: &Callable) -> Result<(), Self> {
        for (i, input_type) in c.inputs.iter().enumerate() {
            if let ResolvedType::Reference(input_type) = input_type {
                if input_type.is_mutable {
                    return Err(CannotTakeMutReferenceError {
                        component_path: c.path.clone(),
                        mut_ref_input_index: i,
                    });
                }
            }
        }
        Ok(())
    }

    pub(crate) fn emit(
        self,
        raw_user_component_id: UserComponentId,
        raw_user_component_db: &UserComponentDb,
        computation_db: &ComputationDb,
        krate_collection: &CrateCollection,
        package_graph: &PackageGraph,
        callable_type: ComponentKind,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
        fn get_snippet(
            callable: &Callable,
            krate_collection: &CrateCollection,
            package_graph: &PackageGraph,
            mut_ref_input_index: usize,
        ) -> Option<AnnotatedSource<NamedSource<String>>> {
            let def = CallableDefinition::compute(callable, krate_collection, package_graph)?;
            let input = &def.sig.inputs[mut_ref_input_index];
            let label = def
                .convert_local_span(input.span())
                .labeled("The &mut input".into());
            Some(AnnotatedSource::new(def.named_source()).label(label))
        }

        let location = raw_user_component_db.get_location(raw_user_component_id);
        let source = diagnostics.source(&location).map(|s| {
            diagnostic::f_macro_span(s.source(), location)
                .labeled(format!("The {callable_type} was registered here"))
                .attach(s)
        });

        let definition_snippet = get_snippet(
            &computation_db[raw_user_component_id],
            krate_collection,
            package_graph,
            self.mut_ref_input_index,
        );
        let diagnostic = CompilerDiagnostic::builder(self)
            .optional_source(source)
            .optional_source(definition_snippet)
            .help(
                "Injected inputs can only be taken by value or via a shared reference (`&`). \
                If you absolutely need to mutate the input, consider internal mutability (e.g. `RefCell`).".into()
            )
            .build();
        diagnostics.push(diagnostic);
    }
}
