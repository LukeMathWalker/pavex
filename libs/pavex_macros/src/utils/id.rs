use super::fn_like::{Callable, ImplContext};
use convert_case::{Case, Casing as _};
use quote::{format_ident, quote, quote_spanned};
use syn::TypeParamBound;

/// Return a path that can be used to link to the callable item in a documentation comment.
///
/// It returns `None` if it can't produce a link that's guaranteed to be valid.
fn callable_doc_link(impl_: Option<&ImplContext>, item: &Callable) -> Option<String> {
    let name = &item.sig.ident;
    let Some(impl_) = impl_ else {
        // A free function.
        return Some(format!("{}()", name));
    };
    if impl_.is_trait_impl {
        return None;
    }
    let ty_name = match impl_.self_ty {
        syn::Type::Path(type_path) => &type_path
            .path
            .segments
            .last()
            .expect("The type path must contains at least one segment, the type name")
            .ident
            .to_string(),
        _ => {
            return None;
        }
    };
    Some(format!("{}::{}()", ty_name, name))
}

fn ty_name(ty_: &syn::Type, current: &mut String) {
    match ty_ {
        syn::Type::Array(type_array) => {
            current.push_str("ARRAY_");
            ty_name(&type_array.elem, current)
        }
        syn::Type::BareFn(type_bare_fn) => {
            current.push_str("FN_");
            ty_name(&type_bare_fn.inputs[0].ty, current)
        }
        syn::Type::Group(type_group) => ty_name(&type_group.elem, current),
        syn::Type::ImplTrait(type_impl_trait) => {
            current.push_str("IMPL_");
            for bound in &type_impl_trait.bounds {
                if let TypeParamBound::Trait(trait_) = bound {
                    let Some(last) = trait_.path.segments.last() else {
                        continue;
                    };
                    current.push_str(&last.ident.to_string());
                    current.push('_');
                }
            }
        }
        syn::Type::Infer(_) => {
            current.push_str("INFERRED");
        }
        syn::Type::Macro(type_macro) => {
            current.push_str("MACRO_");
            current.push_str(
                &type_macro
                    .mac
                    .path
                    .segments
                    .last()
                    .unwrap()
                    .ident
                    .to_string(),
            );
        }
        syn::Type::Never(_) => {
            current.push_str("NEVER");
        }
        syn::Type::Paren(type_paren) => {
            ty_name(&type_paren.elem, current);
        }
        syn::Type::Path(type_path) => {
            if let Some(last) = type_path.path.segments.last() {
                current.push_str(&last.ident.to_string());
                if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                    let type_args = args
                        .args
                        .iter()
                        .filter_map(|arg| {
                            if let syn::GenericArgument::Type(ty) = arg {
                                Some(ty)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                    let n_args = type_args.len();
                    for (i, ty) in type_args.iter().enumerate() {
                        ty_name(ty, current);
                        if i != n_args - 1 {
                            current.push('_');
                        }
                    }
                }
            }
        }
        syn::Type::Ptr(type_ptr) => {
            current.push_str("PTR_");
            ty_name(&type_ptr.elem, current);
        }
        syn::Type::Reference(type_reference) => {
            current.push_str("REF_");
            ty_name(&type_reference.elem, current);
        }
        syn::Type::Slice(type_slice) => {
            current.push_str("SLICE_");
            ty_name(&type_slice.elem, current);
        }
        syn::Type::TraitObject(type_trait_object) => {
            current.push_str("DYN_");
            let trait_bounds: Vec<_> = type_trait_object
                .bounds
                .iter()
                .filter_map(|b| {
                    if let TypeParamBound::Trait(t) = b {
                        Some(t)
                    } else {
                        None
                    }
                })
                .collect();
            let n_bounds = trait_bounds.len();
            for (i, trait_) in trait_bounds.iter().enumerate() {
                if let Some(last) = trait_.path.segments.last() {
                    current.push_str(&last.ident.to_string());
                }
                if i != n_bounds - 1 {
                    current.push('_');
                }
            }
        }
        syn::Type::Tuple(type_tuple) => {
            current.push_str("TUPLE_");
            let n_elements = type_tuple.elems.len();
            for (i, elem) in type_tuple.elems.iter().enumerate() {
                ty_name(elem, current);
                if i != n_elements - 1 {
                    current.push('_');
                }
            }
        }
        unknown => todo!("Unknown `self` type, `{unknown:?}`"),
    }
}

/// Compute the default identifier for a callable item.
pub fn default_id(impl_: Option<&ImplContext>, item: &Callable) -> syn::Ident {
    let name = item.sig.ident.to_string();
    let id = if let Some(impl_) = impl_ {
        let mut buffer = String::new();
        ty_name(impl_.self_ty, &mut buffer);
        buffer.push('_');
        buffer.push_str(&name);
        buffer
    } else {
        name
    }
    .to_case(Case::Constant);
    format_ident!("{id}", span = item.sig.ident.span())
}

/// Return the definition for the id constant for a given callable item.
#[allow(clippy::too_many_arguments)]
pub fn callable_id_def(
    id: &syn::Ident,
    pavex: Option<&syn::Ident>,
    macro_name: &str,
    component_name: &str,
    component_kind: &str,
    registration_method: &str,
    allow_unused: bool,
    impl_: Option<&ImplContext>,
    item: &Callable,
) -> proc_macro2::TokenStream {
    let callable_link = match callable_doc_link(impl_, item) {
        Some(p) => format!("[`{p}`]"),
        None => format!("`{}`", item.sig.ident),
    };
    let callable_path = match callable_doc_link(impl_, item) {
        Some(p) => format!("`{p}`"),
        None => format!("`{}`", item.sig.ident),
    };
    let id_docs = format!(
        r#"A handle to add {callable_link} as {component_kind} to your Pavex application.

# Example

```rust,ignore
use pavex::blueprint::Blueprint;
// [...]
// ^ Import `{id}` here

let mut bp = Blueprint::new();
// Add {callable_path} as {component_kind} to your application.
bp.{registration_method}({id});
```"#
    );
    let pavex = match pavex {
        Some(pavex) => quote! { #pavex },
        None => quote! { ::pavex },
    };
    let allow_unused = allow_unused.then(|| quote! { #[allow(unused)] });
    let component_name = format_ident!("Raw{}", component_name);
    let id_str = id.to_string();
    quote_spanned! { id.span() =>
        #[doc = #id_docs]
        #allow_unused
        pub const #id: #pavex::blueprint::raw::#component_name = #pavex::blueprint::raw::#component_name {
            coordinates: #pavex::blueprint::reflection::AnnotationCoordinates {
                id: #id_str,
                created_at: #pavex::created_at!(),
                macro_name: #macro_name,
            }
        };
    }
}
