//! Utility functions to obtain or manipulate the location where components (constructors,
//! request handlers, etc.) have been registered by the user.
use miette::SourceSpan;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{ExprMethodCall, Stmt};

use pavex_builder::{AppBlueprint, Location, RawCallableIdentifiers};

use crate::diagnostic::{convert_proc_macro_span, ParsedSourceFile, ProcMacroSpanExt};

/// Location, obtained via `#[track_caller]` and `std::panic::Location::caller`, points at the
/// `.` in the method invocation for `route` and `constructor`.
/// E.g.
///
/// ```rust,ignore
/// App::builder()
///   .route(f!(crate::stream_file::<std::path::PathBuf>), "/home")
/// //^ `location` points here!
/// ```
///
/// We want build a `SourceSpan` that matches the `f!` invocation.
/// E.g.
///
/// ```rust,ignore
/// App::builder()
///   .route(f!(crate::stream_file::<std::path::PathBuf>), "/home")
/// //       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
/// //       We want a SourceSpan that points at this!
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
/// is also the smallest node that contains it - exactly what we are looking for.
pub fn get_f_macro_invocation_span(
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
            // This is an optimization - it allows the visitor to skip the entire sub-tree
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
    if let Some(node) = locator.node {
        if let Some(argument) = node.args.first() {
            return Some(convert_proc_macro_span(raw_source, argument.span()));
        }
    }
    None
}

/// Given a callable identifier, return the location where it was registered.
///
/// The same request handlers can be registered multiple times: this function returns the location
/// of the first registration.
pub fn get_registration_location<'a>(
    bp: &'a AppBlueprint,
    identifiers: &RawCallableIdentifiers,
) -> Option<&'a Location> {
    bp.constructor_locations
        .get(identifiers)
        .or_else(|| get_registration_location_for_a_request_handler(bp, identifiers))
        .or_else(|| bp.error_handler_locations.get(identifiers))
}

/// Given the callable identifiers for a request handler, return the location where it was registered.
///
/// The same request handlers can be registered multiple times: this function returns the location
/// of the first registration.
pub fn get_registration_location_for_a_request_handler<'a>(
    bp: &'a AppBlueprint,
    identifiers: &RawCallableIdentifiers,
) -> Option<&'a Location> {
    bp.request_handler_locations
        .get(identifiers)
        .and_then(|v| v.first())
}
