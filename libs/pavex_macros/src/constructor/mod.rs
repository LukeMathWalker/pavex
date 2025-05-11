use crate::utils::validation::must_be_public;
use crate::utils::{CloningStrategy, CloningStrategyFlags, deny_unreachable_pub_attr};
use darling::FromMeta;
use darling::util::Flag;
use lifecycle::Lifecycle;
use proc_macro::TokenStream;
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
    pub error_handler: Option<String>,
}

#[derive(darling::FromMeta, Debug, Clone)]
/// The available options for `#[pavex::request_scoped]`, `#[pavex::transient]`
/// and `#[pavex::singleton]`.
/// Everything in [`InputSchema`], minus `lifecycle`.
pub struct ShorthandSchema {
    pub clone_if_necessary: Flag,
    pub never_clone: Flag,
    pub error_handler: Option<String>,
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
            error_handler,
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
            error_handler,
        })
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
pub struct Properties {
    pub lifecycle: Lifecycle,
    pub cloning_strategy: Option<CloningStrategy>,
    pub error_handler: Option<String>,
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// Everything in [`Properties`], minus `lifecycle`.
pub struct ShorthandProperties {
    pub cloning_strategy: Option<CloningStrategy>,
    pub error_handler: Option<String>,
}

impl TryFrom<ShorthandSchema> for ShorthandProperties {
    type Error = darling::Error;

    fn try_from(input: ShorthandSchema) -> Result<Self, Self::Error> {
        let ShorthandSchema {
            clone_if_necessary,
            never_clone,
            error_handler,
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

        Ok(Self {
            cloning_strategy,
            error_handler,
        })
    }
}

pub fn constructor(metadata: TokenStream, input: TokenStream) -> TokenStream {
    if let Err(e) = reject_invalid_input(input.clone(), "#[pavex::constructor]") {
        return e;
    }
    let attrs = match darling::ast::NestedMeta::parse_meta_list(metadata.into()) {
        Ok(attrs) => attrs,
        Err(err) => return err.to_compile_error().into(),
    };
    let schema = match InputSchema::from_list(&attrs) {
        Ok(parsed) => parsed,
        Err(err) => return err.write_errors().into(),
    };
    let properties = match schema.try_into() {
        Ok(properties) => properties,
        Err(err) => {
            let err: darling::Error = err;
            return err.write_errors().into();
        }
    };
    emit(properties, input)
}

/// An annotation for a request-scoped constructor.
pub fn request_scoped(metadata: TokenStream, input: TokenStream) -> TokenStream {
    shorthand(
        metadata,
        input,
        Lifecycle::RequestScoped,
        "#[pavex::request_scoped]",
    )
}

/// An annotation for a transient constructor.
pub fn transient(metadata: TokenStream, input: TokenStream) -> TokenStream {
    shorthand(metadata, input, Lifecycle::Transient, "#[pavex::transient]")
}

/// An annotation for a singleton constructor.
pub fn singleton(metadata: TokenStream, input: TokenStream) -> TokenStream {
    shorthand(metadata, input, Lifecycle::Singleton, "#[pavex::singleton]")
}

fn shorthand(
    metadata: TokenStream,
    input: TokenStream,
    lifecycle: Lifecycle,
    macro_attr: &'static str,
) -> TokenStream {
    if let Err(e) = reject_invalid_input(input.clone(), macro_attr) {
        return e;
    }
    let attrs = match darling::ast::NestedMeta::parse_meta_list(metadata.into()) {
        Ok(attrs) => attrs,
        Err(err) => return err.to_compile_error().into(),
    };
    let schema = match ShorthandSchema::from_list(&attrs) {
        Ok(parsed) => parsed,
        Err(err) => return err.write_errors().into(),
    };
    let ShorthandProperties {
        cloning_strategy,
        error_handler,
    } = match schema.try_into() {
        Ok(properties) => properties,
        Err(err) => {
            let err: darling::Error = err;
            return err.write_errors().into();
        }
    };
    let properties = Properties {
        lifecycle,
        cloning_strategy,
        error_handler,
    };
    emit(properties, input)
}

/// Decorate the input with a `#[diagnostic::pavex::constructor]` attribute
/// that matches the provided properties.
fn emit(properties: Properties, input: TokenStream) -> TokenStream {
    let Properties {
        lifecycle,
        cloning_strategy,
        error_handler,
    } = properties;
    let mut properties = quote! {
        lifecycle = #lifecycle,
    };
    if let Some(cloning_strategy) = cloning_strategy {
        properties.extend(quote! {
            cloning_strategy = #cloning_strategy,
        });
    }
    if let Some(error_handler) = error_handler {
        properties.extend(quote! {
            error_handler = #error_handler,
        });
    }

    let deny_unreachable_pub = deny_unreachable_pub_attr();

    let input: proc_macro2::TokenStream = input.into();
    quote! {
        #[diagnostic::pavex::constructor(#properties)]
        #deny_unreachable_pub
        #input
    }
    .into()
}

fn reject_invalid_input(input: TokenStream, macro_attr: &'static str) -> Result<(), TokenStream> {
    // Check if the input is a function or a method.
    let (vis, sig) = match (
        syn::parse::<syn::ItemFn>(input.clone()),
        syn::parse::<syn::ImplItemFn>(input.clone()),
    ) {
        (Ok(item_fn), _) => (item_fn.vis, item_fn.sig),
        (_, Ok(impl_fn)) => (impl_fn.vis, impl_fn.sig),
        _ => {
            let msg = format!("{macro_attr} can only be applied to functions and methods.");
            return Err(
                syn::Error::new_spanned(proc_macro2::TokenStream::from(input), msg)
                    .to_compile_error()
                    .into(),
            );
        }
    };
    must_be_public("Constructors", &vis, &sig.ident, &sig)?;
    Ok(())
}
