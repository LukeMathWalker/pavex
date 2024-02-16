mod constructor;
mod error_handler;
mod error_observer;
mod request_handler;
mod wrapping_middleware;

use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::user_components::{UserComponentDb, UserComponentId};
use crate::diagnostic::{
    convert_proc_macro_span, AnnotatedSnippet, CallableDefinition, CallableType,
    CompilerDiagnostic, OptionalSourceSpanExt, SourceSpanExt,
};
use crate::language::{Callable, ResolvedPath};
use crate::rustdoc::CrateCollection;
use crate::{diagnostic, source_or_exit_with_error};
use guppy::graph::PackageGraph;
use syn::spanned::Spanned;

pub(crate) use constructor::{Constructor, ConstructorValidationError};
pub(crate) use error_handler::{ErrorHandler, ErrorHandlerValidationError};
pub(crate) use error_observer::{ErrorObserver, ErrorObserverValidationError};
pub(crate) use request_handler::{RequestHandler, RequestHandlerValidationError};
pub(crate) use wrapping_middleware::{WrappingMiddleware, WrappingMiddlewareValidationError};

#[derive(thiserror::Error, Debug, Clone)]
#[error("You can't inject a mutable reference as an input parameter to `{component_path}`.")]
pub(crate) struct CannotTakeMutReferenceError {
    pub component_path: ResolvedPath,
    pub mut_ref_input_index: usize,
}

impl CannotTakeMutReferenceError {
    pub(crate) fn emit(
        self,
        raw_user_component_id: UserComponentId,
        raw_user_component_db: &UserComponentDb,
        computation_db: &ComputationDb,
        krate_collection: &CrateCollection,
        package_graph: &PackageGraph,
        callable_type: CallableType,
        diagnostics: &mut Vec<miette::Error>,
    ) {
        fn get_snippet(
            callable: &Callable,
            krate_collection: &CrateCollection,
            package_graph: &PackageGraph,
            mut_ref_input_index: usize,
        ) -> Option<AnnotatedSnippet> {
            let def = CallableDefinition::compute(callable, krate_collection, package_graph)?;
            let input = &def.sig.inputs[mut_ref_input_index];
            let label = convert_proc_macro_span(&def.span_contents, input.span())
                .labeled("The &mut input".into());
            Some(AnnotatedSnippet::new(def.named_source(), label))
        }

        let location = raw_user_component_db.get_location(raw_user_component_id);
        let source = source_or_exit_with_error!(location, package_graph, diagnostics);
        let label = diagnostic::get_f_macro_invocation_span(&source, location)
            .labeled(format!("The {callable_type} was registered here"));

        let definition_snippet = get_snippet(
            &computation_db[raw_user_component_id],
            krate_collection,
            package_graph,
            self.mut_ref_input_index,
        );
        let diagnostic = CompilerDiagnostic::builder(source, self)
            .optional_label(label)
            .optional_additional_annotated_snippet(definition_snippet)
            .help(
                "Injected inputs can only be taken by value or via a shared reference (`&`). \
                If you absolutely need to mutate the input, consider internal mutability (e.g. `RefCell`).".into()
            )
            .build();
        diagnostics.push(diagnostic.into());
    }
}
