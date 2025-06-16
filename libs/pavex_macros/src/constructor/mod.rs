use crate::utils::fn_like::{Callable, CallableAnnotation, ImplContext};
use crate::utils::id::{callable_id_def, default_id};
use crate::utils::{AnnotationCodegen, CloningStrategy, CloningStrategyFlags};
use darling::util::Flag;
use lifecycle::Lifecycle;
use quote::quote;

mod lifecycle;

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for `#[pavex::constructor]`.
pub struct InputSchema {
    pub id: Option<syn::Ident>,
    pub pavex: Option<syn::Ident>,
    pub singleton: Flag,
    pub request_scoped: Flag,
    pub transient: Flag,
    pub clone_if_necessary: Flag,
    pub never_clone: Flag,
    pub allow: Option<ConstructorAllows>,
}

#[derive(darling::FromMeta, Debug, Clone)]
pub struct ConstructorAllows {
    unused: Flag,
}

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for `#[pavex::request_scoped]`, `#[pavex::transient]`
/// and `#[pavex::singleton]`.
/// Everything in [`InputSchema`], minus `lifecycle`.
pub struct ShorthandSchema {
    pub id: Option<syn::Ident>,
    pub pavex: Option<syn::Ident>,
    pub clone_if_necessary: Flag,
    pub never_clone: Flag,
    pub allow: Option<ConstructorAllows>,
}

impl TryFrom<InputSchema> for Properties {
    type Error = darling::Error;

    fn try_from(input: InputSchema) -> Result<Self, Self::Error> {
        let InputSchema {
            singleton,
            request_scoped,
            transient,
            clone_if_necessary,
            never_clone,
            id,
            pavex,
            allow,
        } = input;

        let lifecycle = match (
            singleton.is_present(),
            request_scoped.is_present(),
            transient.is_present(),
        ) {
            (true, false, false) => Lifecycle::Singleton,
            (false, true, false) => Lifecycle::RequestScoped,
            (false, false, true) => Lifecycle::Transient,
            (false, false, false) => {
                return Err(darling::Error::custom(
                    "You must specify the lifecycle of your constructor. It can either be `singleton`, `request_scoped`, or `transient`",
                ));
            }
            _ => {
                return Err(darling::Error::custom(
                    "A constructor can't have multiple lifecycles. You can only specify *one* of `singleton`, `request_scoped`, or `transient`.",
                ));
            }
        };

        let Ok(cloning_strategy) = CloningStrategyFlags {
            clone_if_necessary,
            never_clone,
        }
        .try_into() else {
            return Err(darling::Error::custom(
                "A constructor can't have multiple cloning strategies. You can only specify *one* of `never_clone` and `clone_if_necessary`.",
            ));
        };

        let allow_unused = allow.as_ref().map(|a| a.unused.is_present());

        Ok(Properties {
            lifecycle,
            cloning_strategy,
            id,
            pavex,
            allow_unused,
        })
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
pub struct Properties {
    pub lifecycle: Lifecycle,
    pub cloning_strategy: Option<CloningStrategy>,
    pub id: Option<syn::Ident>,
    pub pavex: Option<syn::Ident>,
    pub allow_unused: Option<bool>,
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// Everything in [`Properties`], minus `lifecycle`.
pub struct ShorthandProperties {
    pub cloning_strategy: Option<CloningStrategy>,
    pub id: Option<syn::Ident>,
    pub pavex: Option<syn::Ident>,
    pub allow_unused: Option<bool>,
}

impl TryFrom<ShorthandSchema> for ShorthandProperties {
    type Error = darling::Error;

    fn try_from(input: ShorthandSchema) -> Result<Self, Self::Error> {
        let ShorthandSchema {
            clone_if_necessary,
            never_clone,
            id,
            pavex,
            allow,
        } = input;
        let Ok(cloning_strategy) = CloningStrategyFlags {
            clone_if_necessary,
            never_clone,
        }
        .try_into() else {
            return Err(darling::Error::custom(
                "A constructor can't have multiple cloning strategies. You can only specify *one* of `never_clone` and `clone_if_necessary`.",
            ));
        };
        let allow_unused = allow.as_ref().map(|a| a.unused.is_present());

        Ok(Self {
            cloning_strategy,
            id,
            pavex,
            allow_unused,
        })
    }
}

pub struct ConstructorAnnotataion;

impl CallableAnnotation for ConstructorAnnotataion {
    const PLURAL_COMPONENT_NAME: &str = "Constructors";

    const ATTRIBUTE: &str = "#[pavex::constructor]";

    type InputSchema = InputSchema;

    fn codegen(
        impl_: Option<ImplContext>,
        metadata: Self::InputSchema,
        item: Callable,
    ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
        let properties = metadata
            .try_into()
            .map_err(|e: darling::Error| e.write_errors())?;
        Ok(emit(impl_, item, properties))
    }
}

pub struct RequestScopedAnnotation;

impl CallableAnnotation for RequestScopedAnnotation {
    const PLURAL_COMPONENT_NAME: &str = "Request-scoped constructors";

    const ATTRIBUTE: &str = "#[pavex::request_scoped]";

    type InputSchema = ShorthandSchema;

    fn codegen(
        impl_: Option<ImplContext>,
        metadata: Self::InputSchema,
        item: Callable,
    ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
        shorthand(impl_, metadata, item, Lifecycle::RequestScoped)
    }
}

pub struct TransientAnnotation;

impl CallableAnnotation for TransientAnnotation {
    const PLURAL_COMPONENT_NAME: &str = "Transient constructors";

    const ATTRIBUTE: &str = "#[pavex::transient]";

    type InputSchema = ShorthandSchema;

    fn codegen(
        impl_: Option<ImplContext>,
        metadata: Self::InputSchema,
        item: Callable,
    ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
        shorthand(impl_, metadata, item, Lifecycle::Transient)
    }
}

pub struct SingletonAnnotation;

impl CallableAnnotation for SingletonAnnotation {
    const PLURAL_COMPONENT_NAME: &str = "Singleton constructors";

    const ATTRIBUTE: &str = "#[pavex::singleton]";

    type InputSchema = ShorthandSchema;

    fn codegen(
        impl_: Option<ImplContext>,
        metadata: Self::InputSchema,
        item: Callable,
    ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
        shorthand(impl_, metadata, item, Lifecycle::Singleton)
    }
}

fn shorthand(
    impl_: Option<ImplContext>,
    schema: ShorthandSchema,
    item: Callable,
    lifecycle: Lifecycle,
) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
    let ShorthandProperties {
        cloning_strategy,
        id,
        pavex,
        allow_unused,
    } = schema
        .try_into()
        .map_err(|e: darling::Error| e.write_errors())?;
    let properties = Properties {
        lifecycle,
        cloning_strategy,
        id,
        pavex,
        allow_unused,
    };
    Ok(emit(impl_, item, properties))
}

/// Decorate the input with a `#[diagnostic::pavex::constructor]` attribute
/// that matches the provided properties.
fn emit(impl_: Option<ImplContext>, item: Callable, properties: Properties) -> AnnotationCodegen {
    let Properties {
        lifecycle,
        cloning_strategy,
        id,
        pavex,
        allow_unused,
    } = properties;

    let id = id.unwrap_or_else(|| default_id(impl_.as_ref(), &item));
    let id_str = id.to_string();

    let mut properties = quote! {
        id = #id_str,
        lifecycle = #lifecycle,
    };
    if let Some(cloning_strategy) = cloning_strategy {
        properties.extend(quote! {
            cloning_strategy = #cloning_strategy,
        });
    }
    if let Some(allow_unused) = allow_unused {
        properties.extend(quote! {
            allow_unused = #allow_unused,
        });
    }

    AnnotationCodegen {
        id_def: Some(callable_id_def(
            &id,
            pavex.as_ref(),
            "constructor",
            "Constructor",
            "a constructor",
            "constructor",
            true,
            impl_.as_ref(),
            &item,
        )),
        new_attributes: vec![syn::parse_quote! {
            #[diagnostic::pavex::constructor(#properties)]
        }],
    }
}
