//! Machinery to abstract away the parsing and validation logic that's shared by all
//! Pavex components that accept type-like inputs.
use convert_case::{Case, Casing as _};
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident};
use syn::parse_quote;

use crate::utils::{AnnotationCodegen, deny_unreachable_pub_attr, validation::must_be_public};

/// The item kinds that can be annotated when we expect to be working with a type.
///
/// Its `ToTokens` representation can be used in error spans as the "default" option, unless
/// a more precise span is desired.
pub enum TypeItem {
    Enum(syn::ItemEnum),
    Struct(syn::ItemStruct),
    Use(syn::ItemUse),
}

impl TypeItem {
    fn visibility(&self) -> &syn::Visibility {
        match self {
            TypeItem::Enum(item) => &item.vis,
            TypeItem::Struct(item) => &item.vis,
            TypeItem::Use(item) => &item.vis,
        }
    }

    /// An identifier that can be used of as the "name" of the item.
    pub fn name(&self) -> syn::Ident {
        match self {
            TypeItem::Enum(item_enum) => item_enum.ident.clone(),
            TypeItem::Struct(item_struct) => item_struct.ident.clone(),
            TypeItem::Use(item_use) => {
                let mut current = &item_use.tree;
                loop {
                    match current {
                        syn::UseTree::Path(use_path) => {
                            current = &use_path.tree;
                        }
                        syn::UseTree::Name(use_name) => break use_name.ident.clone(),
                        syn::UseTree::Rename(use_rename) => break use_rename.rename.clone(),
                        syn::UseTree::Glob(_) | syn::UseTree::Group(_) => unreachable!(),
                    }
                }
            }
        }
    }
}

impl ToTokens for TypeItem {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            TypeItem::Enum(item) => {
                item.vis.to_tokens(tokens);
                item.enum_token.to_tokens(tokens);
                item.ident.to_tokens(tokens);
                item.generics.to_tokens(tokens);
            }
            TypeItem::Struct(item) => {
                item.vis.to_tokens(tokens);
                item.struct_token.to_tokens(tokens);
                item.ident.to_tokens(tokens);
                item.generics.to_tokens(tokens);
            }
            TypeItem::Use(item) => {
                item.vis.to_tokens(tokens);
                item.use_token.to_tokens(tokens);
                item.leading_colon.to_tokens(tokens);
                item.tree.to_tokens(tokens);
            }
        }
    }
}

impl TypeItem {
    /// Parse the macro input as either a struct definition, an enum definition or a re-export.
    pub fn parse<M: TypeAnnotation>(input: TokenStream) -> Result<Self, proc_macro::TokenStream> {
        let type_ = if let Ok(struct_) = syn::parse2::<syn::ItemStruct>(input.clone()) {
            TypeItem::Struct(struct_)
        } else if let Ok(enum_) = syn::parse2::<syn::ItemEnum>(input.clone()) {
            TypeItem::Enum(enum_)
        } else if let Ok(use_) = syn::parse2::<syn::ItemUse>(input.clone()) {
            let mut current = &use_.tree;
            loop {
                match current {
                    syn::UseTree::Path(use_path) => {
                        current = &use_path.tree;
                    }
                    syn::UseTree::Name(_) | syn::UseTree::Rename(_) => {
                        break;
                    }
                    syn::UseTree::Glob(_) => {
                        return Err(syn::Error::new_spanned(
                            use_,
                            format!(
                                "Star re-exports can't be annotated with {}.\n\
                            Re-export your {} one by one, annotating each `use` with {}.",
                                M::ATTRIBUTE,
                                M::PLURAL_COMPONENT_NAME.to_case(Case::Lower),
                                M::ATTRIBUTE
                            ),
                        )
                        .to_compile_error()
                        .into());
                    }
                    syn::UseTree::Group(g) => {
                        if g.items.len() > 1 {
                            return Err(syn::Error::new_spanned(
                                use_,
                                format!(
                                    "Grouped re-exports can't be annotated with {}.\n\
                                Re-export your {} one by one, annotating each `use` with {}.",
                                    M::ATTRIBUTE,
                                    M::PLURAL_COMPONENT_NAME.to_case(Case::Lower),
                                    M::ATTRIBUTE
                                ),
                            )
                            .to_compile_error()
                            .into());
                        }
                        current = g.items.first().unwrap();
                    }
                }
            }
            TypeItem::Use(use_)
        } else {
            let msg = format!(
                "{} can only be applied to structs, enums and re-exports.",
                M::ATTRIBUTE
            );
            return Err(syn::Error::new_spanned(input, msg)
                .to_compile_error()
                .into());
        };

        must_be_public(
            M::PLURAL_COMPONENT_NAME,
            type_.visibility(),
            &match &type_ {
                TypeItem::Enum(item) => item.ident.clone(),
                TypeItem::Struct(item) => item.ident.clone(),
                // We need the error message to nudge the user towards marking the _re-export_
                // as `pub`, not the re-exported item.
                // You may wonder: why does the annotated `use` need to be `pub`?
                // Well, if it isn't, it won't show up in the JSON docs for that crate, therefore
                // we won't pick it up.
                TypeItem::Use(_) => format_ident!("use"),
            },
            &type_,
        )?;

        Ok(type_)
    }
}

/// A Pavex annotation that can only be applied to either function or methods.
pub trait TypeAnnotation {
    /// The word(s) to refer to multiple components of this kind.
    /// It must always be capitalized.
    ///
    /// E.g. "Configuration types".
    const PLURAL_COMPONENT_NAME: &str;
    /// The attribute, with its fully qualified path.
    ///
    /// E.g. `#[pavex::config]`.
    const ATTRIBUTE: &str;
    /// The type encoding the expected structure of the macro arguments, flags and options.
    type InputSchema: darling::FromMeta;

    fn codegen(
        metadata: Self::InputSchema,
        item: TypeItem,
    ) -> Result<AnnotationCodegen, proc_macro::TokenStream>;
}

/// The entrypoint used when the input is expected to be free-standing—i.e. not a method
/// within an `impl` block.
pub fn entrypoint<M: TypeAnnotation>(
    metadata: TokenStream,
    input: TokenStream,
) -> proc_macro::TokenStream {
    pub fn _inner<M: TypeAnnotation>(
        metadata: TokenStream,
        input: TokenStream,
    ) -> Result<TokenStream, proc_macro::TokenStream> {
        let type_ = TypeItem::parse::<M>(input.clone())?;
        let allow_unused =
            matches!(type_, TypeItem::Use(_)).then_some(parse_quote! { #[allow(unused)] });

        let mut output = M::codegen(parse_metadata(metadata)?, type_)?;
        output.new_attributes.push(deny_unreachable_pub_attr());
        if let Some(allow_unused) = allow_unused {
            output.new_attributes.push(allow_unused);
        }

        Ok(output.emit(input))
    }

    match _inner::<M>(metadata, input) {
        Ok(t) => t.into(),
        Err(t) => t,
    }
}

fn parse_metadata<T: darling::FromMeta>(metadata: TokenStream) -> Result<T, TokenStream> {
    let attrs =
        darling::ast::NestedMeta::parse_meta_list(metadata).map_err(|e| e.to_compile_error())?;
    T::from_list(&attrs).map_err(|e| e.write_errors())
}
