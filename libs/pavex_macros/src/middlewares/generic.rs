use crate::utils::{
    AnnotationCodegen,
    fn_like::{Callable, ImplContext},
    id::{callable_id_def, default_id},
};
use quote::{format_ident, quote};

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for middleware macros.
pub struct InputSchema {
    pub id: Option<syn::Ident>,
    pub pavex: Option<syn::Ident>,
    pub allow: Option<MiddlewareAllows>,
}

#[derive(darling::FromMeta, Debug, Clone)]
pub struct MiddlewareAllows {
    error_fallback: darling::util::Flag,
}

impl TryFrom<InputSchema> for Properties {
    type Error = darling::Error;

    fn try_from(input: InputSchema) -> Result<Self, Self::Error> {
        let InputSchema { id, pavex, allow } = input;
        let allow_error_fallback = allow.as_ref().map(|a| a.error_fallback.is_present());
        Ok(Properties { id, pavex, allow_error_fallback })
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
pub struct Properties {
    pub id: Option<syn::Ident>,
    pub pavex: Option<syn::Ident>,
    pub allow_error_fallback: Option<bool>,
}

#[derive(Clone, Copy)]
pub enum MiddlewareKind {
    Wrap,
    PreProcess,
    PostProcess,
}

impl MiddlewareKind {
    pub fn macro_name(&self) -> &'static str {
        match self {
            MiddlewareKind::Wrap => "wrap",
            MiddlewareKind::PreProcess => "pre_process",
            MiddlewareKind::PostProcess => "post_process",
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            MiddlewareKind::Wrap => "WrappingMiddleware",
            MiddlewareKind::PreProcess => "PreProcessingMiddleware",
            MiddlewareKind::PostProcess => "PostProcessingMiddleware",
        }
    }
}

pub fn middleware(
    kind: MiddlewareKind,
    impl_: Option<ImplContext>,
    schema: InputSchema,
    item: Callable,
) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
    let properties = schema
        .try_into()
        .map_err(|e: darling::Error| e.write_errors())?;
    Ok(emit(kind, impl_, item, properties))
}

/// Decorate the input with a diagnostic attribute
/// that matches the provided properties.
fn emit(
    kind: MiddlewareKind,
    impl_: Option<ImplContext>,
    item: Callable,
    properties: Properties,
) -> AnnotationCodegen {
    let Properties { id, pavex, allow_error_fallback } = properties;
    let id = id.unwrap_or_else(|| default_id(impl_.as_ref(), &item));
    let id_str = id.to_string();

    let mut properties = quote! {
        id = #id_str,
    };
    
    if let Some(allow_error_fallback) = allow_error_fallback {
        properties.extend(quote! {
            allow_error_fallback = #allow_error_fallback,
        });
    }

    let adj = match kind {
        MiddlewareKind::Wrap => "wrapping",
        MiddlewareKind::PreProcess => "pre-processing",
        MiddlewareKind::PostProcess => "post-processing",
    };
    let bp_method_name = match kind {
        MiddlewareKind::Wrap => "wrap",
        MiddlewareKind::PreProcess => "pre_process",
        MiddlewareKind::PostProcess => "post_process",
    };
    let macro_name = kind.macro_name();
    let id_def = callable_id_def(
        &id,
        pavex.as_ref(),
        macro_name,
        kind.type_name(),
        &format!("a {adj} middleware"),
        bp_method_name,
        false,
        impl_.as_ref(),
        &item,
    );
    let macro_name = format_ident!("{macro_name}");
    AnnotationCodegen {
        id_def: Some(id_def),
        new_attributes: vec![syn::parse_quote! { #[diagnostic::pavex::#macro_name(#properties)] }],
    }
}
