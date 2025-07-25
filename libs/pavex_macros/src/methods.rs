use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{Attribute, ImplItem, visit_mut::VisitMut};

use crate::{
    constructor::{RequestScopedAnnotation, SingletonAnnotation, TransientAnnotation},
    error_handler::ErrorHandlerAnnotation,
    error_observer::ErrorObserverAnnotation,
    fallback::FallbackAnnotation,
    middlewares::{PostProcessAnnotation, PreProcessAnnotation, WrapAnnotation},
    routes::{
        DeleteAnnotation, GetAnnotation, HeadAnnotation, OptionsAnnotation, PatchAnnotation,
        PostAnnotation, PutAnnotation, RouteAnnotation,
    },
    utils::fn_like::{ImplContext, method_entrypoint},
    utils::{AnnotationCodegen, PxStripper},
};

pub fn methods(_metadata: TokenStream, input: TokenStream) -> Result<TokenStream, TokenStream> {
    let mut impl_: syn::ItemImpl = syn::parse(input).map_err(|e| e.into_compile_error())?;
    let mut new_items: Vec<proc_macro2::TokenStream> = Vec::new();

    for item in impl_.items.iter_mut() {
        let ImplItem::Fn(method) = item else {
            continue;
        };
        let Some((sub_attr, attr_index)) =
            method.attrs.iter().enumerate().find_map(|(i, a)| {
                MethodSubAttributes::is_pavex_attr(a).map(|sub_attr| (sub_attr, i))
            })
        else {
            continue;
        };
        let attr = method.attrs.remove(attr_index);
        let metadata = match attr.meta {
            // No arguments, just the path—e.g. `#[error_handler]`.
            syn::Meta::Path(_) => proc_macro2::TokenStream::new(),
            // Arguments within parenthesis, that we want to extract and forward.
            syn::Meta::List(meta_list) => meta_list.tokens.to_token_stream(),
            syn::Meta::NameValue(_) => unreachable!(),
        };

        let impl_context = ImplContext {
            self_ty: impl_.self_ty.as_ref(),
            is_trait_impl: impl_.trait_.is_some(),
        };
        let entrypoint = match sub_attr {
            MethodSubAttributes::ErrorHandler => method_entrypoint::<ErrorHandlerAnnotation>,
            MethodSubAttributes::Route => method_entrypoint::<RouteAnnotation>,
            MethodSubAttributes::Singleton => method_entrypoint::<SingletonAnnotation>,
            MethodSubAttributes::RequestScoped => method_entrypoint::<RequestScopedAnnotation>,
            MethodSubAttributes::Transient => method_entrypoint::<TransientAnnotation>,
            MethodSubAttributes::Get => method_entrypoint::<GetAnnotation>,
            MethodSubAttributes::Post => method_entrypoint::<PostAnnotation>,
            MethodSubAttributes::Put => method_entrypoint::<PutAnnotation>,
            MethodSubAttributes::Patch => method_entrypoint::<PatchAnnotation>,
            MethodSubAttributes::Delete => method_entrypoint::<DeleteAnnotation>,
            MethodSubAttributes::Head => method_entrypoint::<HeadAnnotation>,
            MethodSubAttributes::Options => method_entrypoint::<OptionsAnnotation>,
            MethodSubAttributes::Fallback => method_entrypoint::<FallbackAnnotation>,
            MethodSubAttributes::ErrorObserver => method_entrypoint::<ErrorObserverAnnotation>,
            MethodSubAttributes::PreProcess => method_entrypoint::<PreProcessAnnotation>,
            MethodSubAttributes::PostProcess => method_entrypoint::<PostProcessAnnotation>,
            MethodSubAttributes::Wrap => method_entrypoint::<WrapAnnotation>,
        };
        let AnnotationCodegen {
            id_def,
            new_attributes,
        } = entrypoint(impl_context, metadata, method.to_owned())?;

        if let Some(id_def) = id_def {
            new_items.push(id_def);
        }
        method.attrs.extend(new_attributes);
    }

    PxStripper.visit_item_impl_mut(&mut impl_);

    Ok(quote! {
        #(#new_items)*

        #[diagnostic::pavex::methods]
        #impl_
    }
    .into())
}

/// The kind of attributes that can be applied to methods, and must thus be handled
/// as "helper" sub-attributes of the `#[methods]` macro.
#[derive(Clone, Debug, Copy)]
enum MethodSubAttributes {
    ErrorHandler,
    Singleton,
    RequestScoped,
    Transient,
    Route,
    Get,
    Post,
    Put,
    Patch,
    Options,
    Delete,
    Head,
    Fallback,
    ErrorObserver,
    PreProcess,
    PostProcess,
    Wrap,
}

impl MethodSubAttributes {
    fn attr_name(&self) -> &str {
        use MethodSubAttributes::*;

        match self {
            ErrorHandler => "error_handler",
            Singleton => "singleton",
            RequestScoped => "request_scoped",
            Transient => "transient",
            Route => "route",
            Get => "get",
            Post => "post",
            Put => "put",
            Patch => "patch",
            Options => "options",
            Delete => "delete",
            Head => "head",
            Fallback => "fallback",
            ErrorObserver => "error_observer",
            PreProcess => "pre_process",
            PostProcess => "post_process",
            Wrap => "wrap",
        }
    }

    /// Iterate over the list of sub-attributes.
    fn iter() -> impl Iterator<Item = Self> {
        use MethodSubAttributes::*;

        [
            ErrorHandler,
            Singleton,
            RequestScoped,
            Transient,
            Route,
            Get,
            Post,
            Put,
            Patch,
            Options,
            Delete,
            Head,
            Fallback,
            ErrorObserver,
            PreProcess,
            PostProcess,
            Wrap,
        ]
        .into_iter()
    }

    /// If `attr` is a supported `#[methods]` sub-attribute, its kind will returned.
    /// Otherwise `None` is returned.
    fn is_pavex_attr(attr: &Attribute) -> Option<Self> {
        fn _is_pavex_attr(attr: &Attribute, name: &str) -> bool {
            // Fast‐path for single-segment import form #[error_handler]
            if attr.path().is_ident(name) {
                return true;
            }

            // Multi-segment form #[pavex::error_handler]
            let mut segments = attr.path().segments.iter();
            if let (Some(first), Some(second)) = (segments.next(), segments.next()) {
                if first.ident == "pavex"
                       && second.ident == name
                       // and *no* further segments
                       && segments.next().is_none()
                {
                    return true;
                }
            }

            false
        }

        Self::iter().find(|a| _is_pavex_attr(attr, a.attr_name()))
    }
}
