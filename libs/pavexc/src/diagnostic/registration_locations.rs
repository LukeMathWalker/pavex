//! Utility functions to obtain or manipulate the location where components (constructors,
//! request handlers, etc.) have been registered by the user.
use miette::SourceSpan;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{ExprCall, ExprMethodCall, Stmt};

use pavex::blueprint::reflection::Location;

use crate::diagnostic::{convert_proc_macro_span, ParsedSourceFile, ProcMacroSpanExt};

/// Location, obtained via `#[track_caller]` and `std::panic::Location::caller`, points at the
/// `.` in the method invocation for `route` and `constructor`.
/// E.g.
///
/// ```rust,ignore
/// bp.route(GET, "/home", f!(crate::stream_file::<std::path::PathBuf>))
/// //^ `location` points here!
/// ```
///
/// We build a `SourceSpan` that matches the `f!` invocation.
/// E.g.
///
/// ```rust,ignore
/// bp.route(GET, "/home", f!(crate::stream_file::<std::path::PathBuf>))
/// //                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
/// //                     We want a SourceSpan that points at this for routes
/// bp.constructor(f!(crate::extract_file), Lifecycle::Singleton)
/// //             ^^^^^^^^^^^^^^^^^^^^^^^
/// //             We want a SourceSpan that points at this for constructors
/// ```
pub(crate) fn get_f_macro_invocation_span(
    source: &ParsedSourceFile,
    location: &Location,
) -> Option<SourceSpan> {
    let raw_source = &source.contents;
    let node = find_method_call(location, &source.parsed)?;
    match node {
        Call::MethodCall(node) => {
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
        Call::FunctionCall(_) => {
            tracing::trace!(
                "We do not handle (yet) function call spans when looking for f-macro invoation spans in diagnostics",
            );
            None
        }
    }
}

/// Location, obtained via `#[track_caller]` and `std::panic::Location::caller`, points at the
/// `.` in the method invocation for `route`.
/// E.g.
///
/// ```rust,ignore
/// bp.route(GET, "/home", f!(crate::stream_file::<std::path::PathBuf>))
/// //^ `location` points here!
/// ```
///
/// We build a `SourceSpan` that matches the path argument.
/// E.g.
///
/// ```rust,ignore
/// bp.route(GET, "/home", f!(crate::stream_file::<std::path::PathBuf>))
/// //            ^^^^^^^
/// //            We want a SourceSpan that points at this for routes
/// ```
pub(crate) fn get_route_path_span(
    source: &ParsedSourceFile,
    location: &Location,
) -> Option<SourceSpan> {
    let raw_source = &source.contents;
    let node = find_method_call(location, &source.parsed)?;
    let span = match node {
        Call::MethodCall(node) => {
            let argument = match node.method.to_string().as_str() {
                "route" => {
                    if node.args.len() == 3 {
                        // bp.route(method, path, handler)
                        node.args.iter().nth(1)
                    } else {
                        tracing::trace!("Unexpected number of arguments for `route` invocation");
                        return None;
                    }
                }
                s => {
                    tracing::trace!(
                        "Unknown method name when looking for a `route` invocation: {}",
                        s
                    );
                    return None;
                }
            }?;
            argument.span()
        }
        Call::FunctionCall(node) => {
            let argument = if node.args.len() == 4 {
                // Blueprint::route(bp, method, path, handler)
                node.args.iter().nth(2)
            } else {
                tracing::trace!("Unexpected number of arguments for `route` invocation");
                return None;
            };
            argument.span()
        }
    };
    Some(convert_proc_macro_span(raw_source, span))
}

/// Location, obtained via `#[track_caller]` and `std::panic::Location::caller`, points at the
/// `.` in the method invocation for `nest_at`.
/// E.g.
///
/// ```rust,ignore
/// bp.nest_at("/home", sub_bp)
/// //^ `location` points here!
/// ```
///
/// We build a `SourceSpan` that matches the prefix path argument.
/// E.g.
///
/// ```rust,ignore
/// bp.nest_at("/home", sub_bp)
/// //         ^^^^^^^
/// //         We want a SourceSpan that points at this for nest_at
/// ```
pub(crate) fn get_nest_at_prefix_span(
    source: &ParsedSourceFile,
    location: &Location,
) -> Option<SourceSpan> {
    let raw_source = &source.contents;
    let node = find_method_call(location, &source.parsed)?;
    let span = match node {
        Call::MethodCall(node) => {
            let argument = match node.method.to_string().as_str() {
                "nest_at" => {
                    if node.args.len() == 2 {
                        // bp.nest_at(prefix, sub_bp)
                        node.args.first()
                    } else if node.args.len() == 3 {
                        // Blueprint::nest_at(bp, prefix, sub_bp)
                        node.args.iter().nth(1)
                    } else {
                        tracing::trace!("Unexpected number of arguments for `nest_at` invocation");
                        return None;
                    }
                }
                s => {
                    tracing::trace!(
                        "Unknown method name when looking for a `nest_at` invocation: {}",
                        s
                    );
                    return None;
                }
            }?;
            argument.span()
        }
        Call::FunctionCall(node) => {
            let argument = if node.args.len() == 3 {
                // Blueprint::nest_at(bp, prefix, sub_bp)
                node.args.iter().nth(1)
            } else {
                tracing::trace!("Unexpected number of arguments for `nest_at` invocation");
                return None;
            };
            argument.span()
        }
    };
    Some(convert_proc_macro_span(raw_source, span))
}

/// Location, obtained via `#[track_caller]` and `std::panic::Location::caller`, points at the
/// `(` in the method invocation for `Blueprint::new`.
/// E.g.
///
/// ```rust,ignore
/// let bp = Blueprint::new()
///                      //^ `location` points here!
/// ```
///
/// We build a `SourceSpan` that matches the method call.
/// E.g.
///
/// ```rust,ignore
/// let bp = Blueprint::new()
/// //       ^^^^^^^^^^^^^^
/// //       We want a SourceSpan that points here
/// ```
pub(crate) fn get_bp_new_span(
    source: &ParsedSourceFile,
    location: &Location,
) -> Option<SourceSpan> {
    let raw_source = &source.contents;
    let node = find_method_call(location, &source.parsed)?;
    let Call::FunctionCall(node) = node else { unreachable!() };
    Some(convert_proc_macro_span(raw_source, node.span()))
}

enum Call<'a> {
    MethodCall(&'a ExprMethodCall),
    FunctionCall(&'a ExprCall),
}

/// Visits the abstract syntax tree of a parsed `syn::File`.
/// It looks for a method call node: it tests every method call node to see
/// if `location` falls within its span.
/// It then converts the span associated with the node to a [`SourceSpan`].
///
/// # Ambiguity
///
/// There are going to be multiple nodes that match if we are dealing with chained method calls.
/// Luckily enough, the visit is pre-order, therefore the latest node that contains `location`
/// is also the smallest node that contains it—exactly what we are looking for.
fn find_method_call<'a>(location: &'a Location, file: &'a syn::File) -> Option<Call<'a>> {
    /// A visitor that locates the method call that contains the given `location`.
    struct CallableLocator<'a> {
        location: &'a Location,
        node: Option<Call<'a>>,
    }

    impl<'a> Visit<'a> for CallableLocator<'a> {
        fn visit_expr_call(&mut self, node: &'a ExprCall) {
            if node.span().contains(self.location) {
                self.node = Some(Call::FunctionCall(node));
                syn::visit::visit_expr_call(self, node)
            }
        }

        fn visit_expr_method_call(&mut self, node: &'a ExprMethodCall) {
            if node.span().contains(self.location) {
                self.node = Some(Call::MethodCall(node));
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

    let mut locator = CallableLocator {
        location,
        node: None,
    };
    locator.visit_file(file);
    locator.node
}
