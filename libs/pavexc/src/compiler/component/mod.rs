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
    self, AnnotatedSource, CallableDefSource, CompilerDiagnostic, ComponentKind,
};
use crate::language::{Callable, ResolvedPath, ResolvedType};
use crate::rustdoc::CrateCollection;
use miette::NamedSource;

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
        id: UserComponentId,
        db: &UserComponentDb,
        computation_db: &ComputationDb,
        krate_collection: &CrateCollection,
        callable_type: ComponentKind,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) {
        fn get_snippet(
            callable: &Callable,
            krate_collection: &CrateCollection,
            mut_ref_input_index: usize,
        ) -> Option<AnnotatedSource<NamedSource<String>>> {
            let mut def = CallableDefSource::compute(callable, krate_collection)?;
            def.label_input(mut_ref_input_index, "The &mut input");
            Some(def.annotated_source)
        }

        let source = diagnostics.annotated(
            diagnostic::TargetSpan::Registration(db.registration(id)),
            format!("The {callable_type} was registered here"),
        );

        let definition_snippet = get_snippet(
            &computation_db[id],
            krate_collection,
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
