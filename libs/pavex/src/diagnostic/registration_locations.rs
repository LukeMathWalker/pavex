//! Utility functions to obtain or manipulate the location where components (constructors,
//! request handlers, etc.) have been registered by the user.
use miette::SourceSpan;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{ExprMethodCall, Stmt};

use pavex_builder::Location;

use crate::diagnostic::{convert_proc_macro_span, ParsedSourceFile, ProcMacroSpanExt};

/// Location, obtained via `#[track_caller]` and `std::panic::Location::caller`, points at the
/// `.` in the method invocation for `route` and `constructor`.
/// E.g.
///
/// ```rust,ignore
/// App::builder()
///   .route(GET, "/home", f!(crate::stream_file::<std::path::PathBuf>))
/// //^ `location` points here!
/// ```
///
/// We want build a `SourceSpan` that matches the `f!` invocation.
/// E.g.
///
/// ```rust,ignore
/// App::builder()
///   .route(GET, "/home", f!(crate::stream_file::<std::path::PathBuf>))
/// //                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
/// //                     We want a SourceSpan that points at this for routes
///   .constructor(f!(crate::extract_file), Lifecycle::Singleton)
/// //             ^^^^^^^^^^^^^^^^^^^^^^^
/// //             We want a SourceSpan that points at this for constructors
/// ```
///
/// How do we do it?
/// We parse the source file via `syn` and then visit the abstract syntax tree.
/// We know that we are looking for a method call, so we test every method call node to see
/// if `location` falls within its span.
/// We then convert the span associated with the node to a [`miette::SourceSpan`].
///
/// # Ambiguity
///
/// There are going to be multiple nodes that match if we are dealing with chained method calls.
/// Luckily enough, the visit is pre-order, therefore the latest node that contains `location`
/// is also the smallest node that contains it—exactly what we are looking for.
pub(crate) fn get_f_macro_invocation_span(
    source: &ParsedSourceFile,
    location: &Location,
) -> Option<SourceSpan> {
    struct CallableLocator<'a> {
        location: &'a Location,
        node: Option<&'a ExprMethodCall>,
    }

    impl<'a> Visit<'a> for CallableLocator<'a> {
        fn visit_expr_method_call(&mut self, node: &'a ExprMethodCall) {
            if node.span().contains(self.location) {
                self.node = Some(node);
                syn::visit::visit_expr_method_call(self, node)
            }
        }

        fn visit_stmt(&mut self, node: &'a Stmt) {
            // This is an optimization—it allows the visitor to skip the entire sub-tree
            // under a top-level statement that is not relevant to our search.
            if node.span().contains(self.location) {
                syn::visit::visit_stmt(self, node)
            }
        }
    }

    let raw_source = &source.contents;
    let parsed_source = &source.parsed;
    let mut locator = CallableLocator {
        location,
        node: None,
    };
    locator.visit_file(parsed_source);
    let node = locator.node?;
    let argument = match node.method.to_string().as_str() {
        "error_handler" | "constructor" => node.args.first(),
        "route" => node.args.iter().nth(2),
        s => {
            tracing::trace!(
                "Unknown method name when looking for component registration: {}",
                s
            );
            return None;
        }
    }?;
    Some(convert_proc_macro_span(raw_source, argument.span()))
}
