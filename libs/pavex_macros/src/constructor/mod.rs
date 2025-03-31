use cloning_strategy::CloningStrategy;
use darling::FromMeta;
use darling::util::Flag;
use lifecycle::Lifecycle;
use proc_macro::TokenStream;
use quote::quote;

mod cloning_strategy;
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
        let singleton_present = input.singleton.is_present();
        let request_scoped_present = input.request_scoped.is_present();
        let transient_present = input.transient.is_present();

        let lifecycle = match (singleton_present, request_scoped_present, transient_present) {
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

        let cloning_strategy = match (
            input.never_clone.is_present(),
            input.clone_if_necessary.is_present(),
        ) {
            (true, true) => {
                return Err(darling::Error::custom(
                    "A constructor can't have multiple cloning strategies. You can only specify *one* of `never_clone` and `clone_if_necessary`.",
                ));
            }
            (true, false) => Some(CloningStrategy::NeverClone),
            (false, true) => Some(CloningStrategy::CloneIfNecessary),
            (false, false) => None,
        };

        Ok(Properties {
            lifecycle,
            cloning_strategy,
            error_handler: input.error_handler,
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
        let cloning_strategy = match (
            input.never_clone.is_present(),
            input.clone_if_necessary.is_present(),
        ) {
            (true, true) => {
                return Err(darling::Error::custom(
                    "A constructor can't have multiple cloning strategies. You can only specify *one* of `never_clone` and `clone_if_necessary`.",
                ));
            }
            (true, false) => Some(CloningStrategy::NeverClone),
            (false, true) => Some(CloningStrategy::CloneIfNecessary),
            (false, false) => None,
        };

        Ok(Self {
            cloning_strategy,
            error_handler: input.error_handler,
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

    let input: proc_macro2::TokenStream = input.into();
    quote! {
        #[diagnostic::pavex::constructor(#properties)]
        #input
    }
    .into()
}

fn reject_invalid_input(input: TokenStream, macro_attr: &'static str) -> Result<(), TokenStream> {
    // Check if the input is a function
    if syn::parse::<syn::ItemFn>(input.clone()).is_ok() {
        return Ok(());
    };
    if syn::parse::<syn::ImplItemFn>(input.clone()).is_ok() {
        return Ok(());
    }

    // Neither ItemFn nor ImplItemFn - return an error
    let msg = format!("{macro_attr} can only be applied to functions and methods.");
    Err(
        syn::Error::new_spanned(proc_macro2::TokenStream::from(input), msg)
            .to_compile_error()
            .into(),
    )
}
