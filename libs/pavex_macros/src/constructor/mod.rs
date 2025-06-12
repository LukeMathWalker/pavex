use crate::fn_like::{Callable, CallableAnnotation, ImplContext};
use crate::utils::{AnnotationCodegen, CloningStrategy, CloningStrategyFlags};
use darling::util::Flag;
use lifecycle::Lifecycle;
use quote::quote;

mod lifecycle;

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for `#[pavex::constructor]`.
pub struct InputSchema {
    pub singleton: Flag,
    pub request_scoped: Flag,
    pub transient: Flag,
    pub clone_if_necessary: Flag,
    pub never_clone: Flag,
}

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for `#[pavex::request_scoped]`, `#[pavex::transient]`
/// and `#[pavex::singleton]`.
/// Everything in [`InputSchema`], minus `lifecycle`.
pub struct ShorthandSchema {
    pub clone_if_necessary: Flag,
    pub never_clone: Flag,
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

        Ok(Properties {
            lifecycle,
            cloning_strategy,
        })
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
pub struct Properties {
    pub lifecycle: Lifecycle,
    pub cloning_strategy: Option<CloningStrategy>,
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// Everything in [`Properties`], minus `lifecycle`.
pub struct ShorthandProperties {
    pub cloning_strategy: Option<CloningStrategy>,
}

impl TryFrom<ShorthandSchema> for ShorthandProperties {
    type Error = darling::Error;

    fn try_from(input: ShorthandSchema) -> Result<Self, Self::Error> {
        let ShorthandSchema {
            clone_if_necessary,
            never_clone,
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

        Ok(Self { cloning_strategy })
    }
}

pub struct ConstructorAnnotataion;

impl CallableAnnotation for ConstructorAnnotataion {
    const PLURAL_COMPONENT_NAME: &str = "Constructors";

    const ATTRIBUTE: &str = "#[pavex::constructor]";

    type InputSchema = InputSchema;

    fn codegen(
        _impl_: Option<ImplContext>,
        metadata: Self::InputSchema,
        _item: Callable,
    ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
        let properties = metadata
            .try_into()
            .map_err(|e: darling::Error| e.write_errors())?;
        Ok(emit(properties))
    }
}

pub struct RequestScopedAnnotation;

impl CallableAnnotation for RequestScopedAnnotation {
    const PLURAL_COMPONENT_NAME: &str = "Request-scoped constructors";

    const ATTRIBUTE: &str = "#[pavex::request_scoped]";

    type InputSchema = ShorthandSchema;

    fn codegen(
        _impl_: Option<ImplContext>,
        metadata: Self::InputSchema,
        _item: Callable,
    ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
        shorthand(metadata, Lifecycle::RequestScoped)
    }
}

pub struct TransientAnnotation;

impl CallableAnnotation for TransientAnnotation {
    const PLURAL_COMPONENT_NAME: &str = "Transient constructors";

    const ATTRIBUTE: &str = "#[pavex::transient]";

    type InputSchema = ShorthandSchema;

    fn codegen(
        _impl_: Option<ImplContext>,
        metadata: Self::InputSchema,
        _item: Callable,
    ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
        shorthand(metadata, Lifecycle::Transient)
    }
}

pub struct SingletonAnnotation;

impl CallableAnnotation for SingletonAnnotation {
    const PLURAL_COMPONENT_NAME: &str = "Singleton constructors";

    const ATTRIBUTE: &str = "#[pavex::singleton]";

    type InputSchema = ShorthandSchema;

    fn codegen(
        _impl_: Option<ImplContext>,
        metadata: Self::InputSchema,
        _item: Callable,
    ) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
        shorthand(metadata, Lifecycle::Singleton)
    }
}

fn shorthand(
    schema: ShorthandSchema,
    lifecycle: Lifecycle,
) -> Result<AnnotationCodegen, proc_macro::TokenStream> {
    let ShorthandProperties { cloning_strategy } = schema
        .try_into()
        .map_err(|e: darling::Error| e.write_errors())?;
    let properties = Properties {
        lifecycle,
        cloning_strategy,
    };
    Ok(emit(properties))
}

/// Decorate the input with a `#[diagnostic::pavex::constructor]` attribute
/// that matches the provided properties.
fn emit(properties: Properties) -> AnnotationCodegen {
    let Properties {
        lifecycle,
        cloning_strategy,
    } = properties;
    let mut properties = quote! {
        lifecycle = #lifecycle,
    };
    if let Some(cloning_strategy) = cloning_strategy {
        properties.extend(quote! {
            cloning_strategy = #cloning_strategy,
        });
    }

    AnnotationCodegen {
        id_def: None,
        new_attributes: vec![syn::parse_quote! {
            #[diagnostic::pavex::constructor(#properties)]
        }],
    }
}
