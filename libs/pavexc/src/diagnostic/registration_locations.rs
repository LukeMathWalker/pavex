//! Utility functions to obtain or manipulate the location where components (constructors,
//! request handlers, etc.) have been registered by the user.
use miette::SourceSpan;
use proc_macro2::Span;
use std::ops::Deref;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprMethodCall, ImplItemFn, ItemFn, Stmt};

use pavex_bp_schema::Location;

use super::{ComponentKind, Registration, RegistrationKind};
use crate::diagnostic::{ParsedSourceFile, ProcMacroSpanExt, convert_proc_macro_span};

/// Returns a span covering the registration location.
///
/// For attributes, returns a span covering the attribute (e.g. `#[pavex::constructor]`) as well as the function/method
/// signature.
/// For blueprint registrations, returns a span pointing at the method argument that accepts the
/// raw identifiers for that component.
pub(crate) fn registration_span(
    source: &ParsedSourceFile,
    registration: &Registration,
    kind: ComponentKind,
) -> Option<SourceSpan> {
    match registration.kind {
        RegistrationKind::Blueprint => f_macro_span(source, &registration.location),
        RegistrationKind::Attribute => attribute_span(source, &registration.location, kind),
    }
}

/// A span covering the attribute (e.g. `#[pavex::constructor]`) as well as the function/method
/// signature.
pub(crate) fn attribute_span(
    source: &ParsedSourceFile,
    location: &Location,
    kind: ComponentKind,
) -> Option<SourceSpan> {
    if kind == ComponentKind::ErrorHandler {
        return attribute_error_handler_span(source, location);
    }

    let raw_source = &source.contents;
    let span = if kind == ComponentKind::ConfigType || kind == ComponentKind::PrebuiltType {
        if let Some(type_def) = find_type_def(location, &source.parsed) {
            type_def.attrs_and_name_span()
        } else if let Some(use_) = find_use(location, &source.parsed) {
            use_.span()
        } else {
            return None;
        }
    } else {
        find_callable_def(location, &source.parsed)?.attrs_and_sig_span()
    };
    Some(convert_proc_macro_span(raw_source, span))
}

/// A span matching the `error_handler = "error::error::path"` portion of an attribute.
pub(crate) fn attribute_error_handler_span(
    source: &ParsedSourceFile,
    location: &Location,
) -> Option<SourceSpan> {
    use darling::FromMeta;

    let raw_source = &source.contents;
    let def = find_callable_def(location, &source.parsed)?;
    let attrs = match &def {
        CallableDef::Method(m) => &m.attrs,
        CallableDef::Function(f) => &f.attrs,
    };

    #[derive(darling::FromMeta)]
    struct Property {
        error_handler: darling::util::SpannedValue<String>,
    }

    for attr in attrs {
        if let Ok(property) = Property::from_meta(&attr.meta) {
            return Some(convert_proc_macro_span(
                raw_source,
                property.error_handler.span(),
            ));
        }
    }

    // Fall back to the full sign+attr span if we can't find a precise one.
    Some(convert_proc_macro_span(
        raw_source,
        def.attrs_and_sig_span(),
    ))
}

/// A span matching the `path = "/my/route/path"` portion of a route attribute.
pub(crate) fn route_path_attr_span(
    source: &ParsedSourceFile,
    location: &Location,
) -> Option<SourceSpan> {
    use darling::FromMeta;

    let raw_source = &source.contents;
    let def = find_callable_def(location, &source.parsed)?;
    let attrs = match &def {
        CallableDef::Method(m) => &m.attrs,
        CallableDef::Function(f) => &f.attrs,
    };

    #[derive(darling::FromMeta)]
    struct Property {
        path: darling::util::SpannedValue<String>,
    }

    for attr in attrs {
        if let Ok(property) = Property::from_meta(&attr.meta) {
            return Some(convert_proc_macro_span(raw_source, property.path.span()));
        }
    }

    // Fall back to the full sign+attr span if we can't find a precise one.
    Some(convert_proc_macro_span(
        raw_source,
        def.attrs_and_sig_span(),
    ))
}

/// A span matching the `impl MyType` portion of an impl item.
pub(crate) fn impl_header_span(
    source: &ParsedSourceFile,
    location: &Location,
) -> Option<SourceSpan> {
    let raw_source = &source.contents;
    let impl_ = find_impl(location, &source.parsed)?;

    let mut span = impl_.impl_token.span();
    // Expand the span to everything until the `{` starting the impl block.
    span = span.join(impl_.brace_token.span.open()).unwrap_or(span);

    // Fall back to the full sign+attr span if we can't find a precise one.
    Some(convert_proc_macro_span(raw_source, span))
}

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
pub(crate) fn f_macro_span(source: &ParsedSourceFile, location: &Location) -> Option<SourceSpan> {
    let raw_source = &source.contents;
    let node = find_method_call(location, &source.parsed)?;
    match node {
        Call::MethodCall(node) => {
            let argument_index = match node.method.to_string().as_str() {
                "error_handler" | "error_observer" | "constructor" | "wrap" | "pre_process"
                | "post_process" | "fallback" | "singleton" | "request_scoped" | "transient"
                | "prebuilt" => 0,
                "config" => 1,
                "route" => 2,
                s => {
                    tracing::trace!(
                        "Unknown method name when looking for component registration: {}",
                        s
                    );
                    return None;
                }
            };
            let argument = node.args.iter().nth(argument_index)?;
            Some(convert_proc_macro_span(raw_source, argument.span()))
        }
        Call::FunctionCall(node) => {
            if let Expr::Path(path) = node.func.deref() {
                let segments = &path.path.segments;
                if segments.len() >= 2 {
                    let method_name = segments[segments.len() - 1].ident.to_string();
                    let type_name = segments[segments.len() - 2].ident.to_string();
                    let index = match (type_name.as_str(), method_name.as_str()) {
                        ("Blueprint", "error_handler")
                        | ("Blueprint", "error_observer")
                        | ("Blueprint", "constructor")
                        | ("Blueprint", "singleton")
                        | ("Blueprint", "request_scoped")
                        | ("Blueprint", "transient")
                        | ("Blueprint", "wrap")
                        | ("Blueprint", "pre_process")
                        | ("Blueprint", "post_process")
                        | ("Blueprint", "prebuilt")
                        | ("Blueprint", "fallback")
                        | ("ConfigType", "new") => {
                            // Blueprint::error_handler(bp, handler)
                            // Blueprint::error_observer(bp, observer)
                            // Blueprint::constructor(bp, constructor, lifecycle)
                            // Blueprint::singleton(bp, constructor)
                            // Blueprint::request_scoped(bp, constructor)
                            // Blueprint::transient(bp, constructor)
                            // Blueprint::wrap(bp, middleware)
                            // Blueprint::pre_process(bp, middleware)
                            // Blueprint::post_process(bp, middleware)
                            // Blueprint::fallback(bp, fallback)
                            // Blueprint::prebuilt(bp, prebuilt)
                            // ConfigType::new(key, config)
                            1
                        }
                        ("Blueprint", "route") => {
                            // Blueprint::route(bp, method, path_pattern, handler)
                            3
                        }
                        ("Route", "new") | ("Blueprint", "config") => {
                            // Blueprint::config(bp, key, config)
                            // Route::new(method, path_pattern, handler)
                            2
                        }
                        ("Constructor", "new")
                        | ("Constructor", "request_scoped")
                        | ("Constructor", "transient")
                        | ("Constructor", "singleton")
                        | ("WrappingMiddleware", "new")
                        | ("PreProcessingMiddleware", "new")
                        | ("PostProcessingMiddleware", "new")
                        | ("ErrorObserver", "new")
                        | ("PrebuiltType", "new")
                        | ("Fallback", "new") => {
                            // Constructor::new(constructor, lifecycle)
                            // Constructor::request_scoped(constructor)
                            // Constructor::transient(constructor)
                            // Constructor::singleton(constructor)
                            // WrappingMiddleware::new(mw)
                            // PreProcessingMiddleware::new(mw)
                            // PostProcessingMiddleware::new(mw)
                            // ErrorObserver::new(observer)
                            // Fallback::new(fallback)
                            // PrebuiltType::new(prebuilt)
                            0
                        }
                        _ => {
                            tracing::trace!(
                            node = ?node,
                            "We couldn't extract an f-macro invocation span from this function call node",
                            );
                            return None;
                        }
                    };
                    return node
                        .args
                        .iter()
                        .nth(index)
                        .map(|argument| convert_proc_macro_span(raw_source, argument.span()));
                }
            }
            tracing::trace!(
                node = ?node,
                "We couldn't extract an f-macro invocation span from this function call node",
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
pub(crate) fn route_path_span(
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

/// Returns a span covering the registration location.
///
/// For attributes, returns a span covering the key property (e.g. `key = "my-config-key"`).
/// For blueprint registrations, returns a span pointing at the method argument that accepts key.
pub(crate) fn config_key_span(
    source: &ParsedSourceFile,
    registration: &Registration,
) -> Option<SourceSpan> {
    match registration.kind {
        RegistrationKind::Blueprint => bp_config_key_span(source, &registration.location),
        RegistrationKind::Attribute => attribute_config_key_span(source, &registration.location),
    }
}

/// A span matching the `key = "my-config-key"` portion of an attribute.
pub(crate) fn attribute_config_key_span(
    source: &ParsedSourceFile,
    location: &Location,
) -> Option<SourceSpan> {
    use darling::FromMeta;
    let raw_source = &source.contents;
    let def = find_type_def(location, &source.parsed)?;
    let attrs = match &def {
        TypeDef::Struct(s) => &s.attrs,
        TypeDef::Enum(e) => &e.attrs,
    };

    #[derive(darling::FromMeta)]
    struct Property {
        key: darling::util::SpannedValue<String>,
    }

    for attr in attrs {
        if let Ok(property) = Property::from_meta(&attr.meta) {
            return Some(convert_proc_macro_span(raw_source, property.key.span()));
        }
    }

    // Fall back to the full name+attr span if we can't find a precise one.
    Some(convert_proc_macro_span(
        raw_source,
        def.attrs_and_name_span(),
    ))
}

/// Location, obtained via `#[track_caller]` and `std::panic::Location::caller`, points at the
/// `.` in the method invocation for `config`.
/// E.g.
///
/// ```rust,ignore
/// bp.config("home", t!(crate::Streamer))
/// //^ `location` points here!
/// ```
///
/// We build a `SourceSpan` that matches the key argument.
/// E.g.
///
/// ```rust,ignore
/// bp.config("home", t!(crate::Streamer))
/// //        ^^^^^^
/// //        We want a SourceSpan that points at this for routes
/// ```
pub(crate) fn bp_config_key_span(
    source: &ParsedSourceFile,
    location: &Location,
) -> Option<SourceSpan> {
    let raw_source = &source.contents;
    let node = find_method_call(location, &source.parsed)?;
    let span = match node {
        Call::MethodCall(node) => {
            let argument = match node.method.to_string().as_str() {
                "config" => {
                    if node.args.len() == 2 {
                        // bp.config(key, type)
                        node.args.iter().next()
                    } else {
                        tracing::trace!("Unexpected number of arguments for `config` invocation");
                        return None;
                    }
                }
                s => {
                    tracing::trace!(
                        "Unknown method name when looking for a `config` invocation: {}",
                        s
                    );
                    return None;
                }
            }?;
            argument.span()
        }
        Call::FunctionCall(node) => {
            let argument = if node.args.len() == 3 {
                // Blueprint::config(bp, key, type)
                node.args.iter().nth(1)
            } else {
                tracing::trace!("Unexpected number of arguments for `config` invocation");
                return None;
            };
            argument.span()
        }
    };
    Some(convert_proc_macro_span(raw_source, span))
}

/// Location, obtained via `#[track_caller]` and `std::panic::Location::caller`, points at the
/// `.` in the method invocation for `prefix`.
/// E.g.
///
/// ```rust,ignore
/// bp.prefix("/home")
/// //^ `location` points here!
/// ```
///
/// We build a `SourceSpan` that matches the prefix path argument.
/// E.g.
///
/// ```rust,ignore
/// bp.prefix("/home")
/// //        ^^^^^^^
/// //        We want a SourceSpan that points at this
/// ```
pub(crate) fn prefix_span(source: &ParsedSourceFile, location: &Location) -> Option<SourceSpan> {
    let arguments = get_inherent_method_arguments("prefix", source, location)?;
    Some(convert_proc_macro_span(
        &source.contents,
        arguments.first()?.span(),
    ))
}

/// Location, obtained via `#[track_caller]` and `std::panic::Location::caller`, points at the
/// `.` in the method invocation for `domain`.
/// E.g.
///
/// ```rust,ignore
/// bp.domain("example.com")
/// //^ `location` points here!
/// ```
///
/// We build a `SourceSpan` that matches the domain argument.
/// E.g.
///
/// ```rust,ignore
/// bp.domain("bp.com")
/// //        ^^^^^^^
/// //        We want a SourceSpan that points at this
/// ```
pub(crate) fn domain_span(source: &ParsedSourceFile, location: &Location) -> Option<SourceSpan> {
    let arguments = get_inherent_method_arguments("domain", source, location)?;
    Some(convert_proc_macro_span(
        &source.contents,
        arguments.first()?.span(),
    ))
}

/// Location, obtained via `#[track_caller]` and `std::panic::Location::caller`, points at the
/// `.` in the method invocation for `import`.
/// E.g.
///
/// ```rust,ignore
/// bp.import(from![*])
/// //^ `location` points here!
/// ```
///
/// We build a `SourceSpan` that matches the sources argument.
/// E.g.
///
/// ```rust,ignore
/// bp.import(from![*])
/// //        ^^^^^^^
/// //        We want a SourceSpan that points at this
/// ```
pub(crate) fn imported_sources_span(
    source: &ParsedSourceFile,
    location: &Location,
) -> Option<SourceSpan> {
    let arguments = get_inherent_method_arguments("import", source, location)?;
    Some(convert_proc_macro_span(
        &source.contents,
        arguments.first()?.span(),
    ))
}

/// Location, obtained via `#[track_caller]` and `std::panic::Location::caller`, points at the
/// `.` in the method invocation for `nest`.
/// E.g.
///
/// ```rust,ignore
/// bp.nest(sub_bp)
/// //^ `location` points here!
/// ```
///
/// We build a `SourceSpan` that matches the blueprint argument.
/// E.g.
///
/// ```rust,ignore
/// bp.nest(sub_bp)
/// //      ^^^^^^
/// //      We want a SourceSpan that points at this for nest
/// ```
pub(crate) fn nest_blueprint_span(
    source: &ParsedSourceFile,
    location: &Location,
) -> Option<SourceSpan> {
    let arguments = get_inherent_method_arguments("nest", source, location)?;
    Some(convert_proc_macro_span(
        &source.contents,
        arguments.first()?.span(),
    ))
}

/// Location, obtained via `#[track_caller]` and `std::panic::Location::caller`, points at the
/// `.` in the method invocation.
/// E.g.
///
/// ```rust,ignore
/// bp.prefix("/home")
/// //^ `location` points here!
/// ```
///
/// We return the arguments of the invocation. If the inherent is invoked as a static method
/// (i.e. `Blueprint::prefix`), we skip the first argument (the blueprint, `self`).
fn get_inherent_method_arguments(
    method_name: &str,
    source: &ParsedSourceFile,
    location: &Location,
) -> Option<Vec<Expr>> {
    let node = find_method_call(location, &source.parsed)?;
    match node {
        Call::MethodCall(node) => {
            let found = node.method.to_string();
            if found.as_str() == method_name {
                Some(node.args.iter().cloned().collect())
            } else {
                tracing::trace!(
                    "Unknown method name when looking for a `{}` invocation: {}",
                    method_name,
                    found
                );
                None
            }
        }
        Call::FunctionCall(node) => Some(node.args.iter().skip(1).cloned().collect()),
    }
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
pub(crate) fn bp_new_span(source: &ParsedSourceFile, location: &Location) -> Option<SourceSpan> {
    let raw_source = &source.contents;
    let node = find_method_call(location, &source.parsed)?;
    let Call::FunctionCall(node) = node else {
        return None;
    };
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
    struct CallLocator<'a> {
        location: &'a Location,
        node: Option<Call<'a>>,
    }

    impl<'a> Visit<'a> for CallLocator<'a> {
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

    let mut locator = CallLocator {
        location,
        node: None,
    };
    locator.visit_file(file);
    locator.node
}

enum CallableDef<'a> {
    Method(&'a ImplItemFn),
    Function(&'a ItemFn),
}

impl CallableDef<'_> {
    /// Return a span that includes the attributes and signature of the callable definition.
    fn attrs_and_sig_span(&self) -> Span {
        let (attrs, vis, sig) = match self {
            CallableDef::Method(item) => (&item.attrs, &item.vis, &item.sig),
            CallableDef::Function(item) => (&item.attrs, &item.vis, &item.sig),
        };

        let mut span = attrs
            .iter()
            .find(|a| a.path().segments.first().map(|s| s.ident.to_string()) != Some("doc".into()))
            .map(|attr| attr.span())
            .unwrap_or_else(|| vis.span());
        // Expand the span to include visibility
        span = span.join(vis.span()).unwrap_or(span);
        // Expand to include the function signature
        span = span.join(sig.output.span()).unwrap_or(span);

        span
    }
}

/// Visits the abstract syntax tree of a parsed `syn::File` to find a function definition or method
/// definition node that contains the given `location`.
/// It then converts the span associated with the node to a [`SourceSpan`].
///
/// # Ambiguity
///
/// There may be multiple nodes that match (e.g. a function defined inside another function).
/// Luckily enough, the visit is pre-order, therefore the latest node that contains `location`
/// is also the smallest node that contains it—exactly what we are looking for.
fn find_callable_def<'a>(location: &'a Location, file: &'a syn::File) -> Option<CallableDef<'a>> {
    /// A visitor that locates the method call that contains the given `location`.
    struct CallableLocator<'a> {
        location: &'a Location,
        node: Option<CallableDef<'a>>,
    }

    impl<'a> Visit<'a> for CallableLocator<'a> {
        fn visit_item_fn(&mut self, node: &'a syn::ItemFn) {
            if node.span().contains(self.location) {
                self.node = Some(CallableDef::Function(node));
                syn::visit::visit_item_fn(self, node)
            }
        }
        fn visit_impl_item_fn(&mut self, node: &'a syn::ImplItemFn) {
            if node.span().contains(self.location) {
                self.node = Some(CallableDef::Method(node));
                syn::visit::visit_impl_item_fn(self, node)
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

enum TypeDef<'a> {
    Struct(&'a syn::ItemStruct),
    Enum(&'a syn::ItemEnum),
}

impl TypeDef<'_> {
    /// Return a span that includes the attributes and name of the type.
    fn attrs_and_name_span(&self) -> Span {
        let (attrs, vis, ident) = match self {
            TypeDef::Struct(item) => (&item.attrs, &item.vis, &item.ident),
            TypeDef::Enum(item) => (&item.attrs, &item.vis, &item.ident),
        };

        let mut span = attrs
            .first()
            .map(|attr| attr.span())
            .unwrap_or_else(|| vis.span());
        // Expand the span to include visibility
        span = span.join(vis.span()).unwrap_or(span);
        // Expand to include the type name
        span = span.join(ident.span()).unwrap_or(span);

        span
    }
}

/// Visits the abstract syntax tree of a parsed `syn::File` to find a struct or enum definition
/// that contains the given `location`.
/// It then converts the span associated with the node to a [`SourceSpan`].
fn find_type_def<'a>(location: &'a Location, file: &'a syn::File) -> Option<TypeDef<'a>> {
    /// A visitor that locates the type def that contains the given `location`.
    struct TypeLocator<'a> {
        location: &'a Location,
        node: Option<TypeDef<'a>>,
    }

    impl<'a> Visit<'a> for TypeLocator<'a> {
        fn visit_item_struct(&mut self, i: &'a syn::ItemStruct) {
            if i.span().contains(self.location) {
                self.node = Some(TypeDef::Struct(i));
            }
        }
        fn visit_item_enum(&mut self, i: &'a syn::ItemEnum) {
            if i.span().contains(self.location) {
                self.node = Some(TypeDef::Enum(i));
            }
        }
    }

    let mut locator = TypeLocator {
        location,
        node: None,
    };
    locator.visit_file(file);
    locator.node
}

/// Visits the abstract syntax tree of a parsed `syn::File` to find a `use` statement node
/// that contains the given `location`.
/// It then converts the span associated with the node to a [`SourceSpan`].
fn find_use<'a>(location: &'a Location, file: &'a syn::File) -> Option<&'a syn::ItemUse> {
    /// A visitor that locates the `use` statement that contains the given `location`.
    struct UseLocator<'a> {
        location: &'a Location,
        node: Option<&'a syn::ItemUse>,
    }

    impl<'a> Visit<'a> for UseLocator<'a> {
        fn visit_item_use(&mut self, node: &'a syn::ItemUse) {
            if node.span().contains(self.location) {
                self.node = Some(node);
                syn::visit::visit_item_use(self, node)
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

    let mut locator = UseLocator {
        location,
        node: None,
    };
    locator.visit_file(file);
    locator.node
}

/// Visits the abstract syntax tree of a parsed `syn::File` to find an impl block node
/// that contains the given `location`.
/// It then converts the span associated with the node to a [`SourceSpan`].
///
/// # Ambiguity
///
/// There may be multiple nodes that match (e.g. an impl defined inside another impl).
/// Luckily enough, the visit is pre-order, therefore the latest node that contains `location`
/// is also the smallest node that contains it—exactly what we are looking for.
fn find_impl<'a>(location: &'a Location, file: &'a syn::File) -> Option<&'a syn::ItemImpl> {
    /// A visitor that locates the impl def that contains the given `location`.
    struct Locator<'a> {
        location: &'a Location,
        node: Option<&'a syn::ItemImpl>,
    }

    impl<'a> Visit<'a> for Locator<'a> {
        fn visit_item_impl(&mut self, node: &'a syn::ItemImpl) {
            if node.span().contains(self.location) {
                self.node = Some(node);
                syn::visit::visit_item_impl(self, node)
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

    let mut locator = Locator {
        location,
        node: None,
    };
    locator.visit_file(file);
    locator.node
}
