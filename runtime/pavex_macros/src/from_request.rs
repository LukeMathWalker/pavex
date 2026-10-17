#![allow(unused)]
use std::collections::{BTreeMap, BTreeSet};

use darling::{
    FromDeriveInput, FromField, FromMeta,
    util::{Flag, Ignored},
};
use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{DeriveInput, Ident, Type, parse_macro_input, spanned::Spanned};

#[derive(FromDeriveInput)]
#[darling(supports(struct_named))]
struct FromRequestInput {
    // Pull out all named fields and let darling map each one via `Field`.
    data: darling::ast::Data<Ignored, ParsedField>,
    generics: syn::Generics,
    ident: syn::Ident,
    vis: syn::Visibility,
}

#[derive(Debug)]
struct ParsedField {
    ident: Ident,
    ty: Type,
    source: FieldSource,
}

// What we actually want to use in codegen per field.
#[derive(Debug)]
enum FieldSource {
    PathParam(PathParam),
    PathParams,
    QueryParam(QueryParam),
    QueryParams,
    Header(Header),
    Headers,
    Body(Body),
}

// Nested meta for `#[path_param(name = "...")]`
#[derive(Default, Debug, FromMeta)]
#[darling(default, from_word = || Ok(Default::default()))]
struct PathParam {
    name: Option<String>,
}

// Nested meta for `#[query_param(name = "...")]`
#[derive(Default, Debug, FromMeta)]
#[darling(default, from_word = || Ok(Default::default()))]
struct QueryParam {
    name: Option<String>,
}

// Nested meta for `#[header(name = "...")]`
#[derive(Default, Debug, FromMeta)]
#[darling(default, from_word = || Ok(Default::default()))]
struct Header {
    name: Option<String>,
}

// Nested meta for `#[body(format = "...")]`
#[derive(Debug, FromMeta)]
#[darling(from_word = || Err(darling::Error::custom("You must specify the expected body encoding via `format`.\nFor example, `#[body(format = \"json\")]`.")))]
struct Body {
    format: String,
}

impl FromField for ParsedField {
    fn from_field(field: &syn::Field) -> darling::Result<Self> {
        let mut sources = Vec::new();
        for attr in &field.attrs {
            let ident = attr.path().require_ident()?.to_string();
            let source = match ident.as_str() {
                "path_param" => FieldSource::PathParam(PathParam::from_meta(&attr.meta)?),
                "query_param" => FieldSource::QueryParam(QueryParam::from_meta(&attr.meta)?),
                "header" => FieldSource::Header(Header::from_meta(&attr.meta)?),
                "body" => FieldSource::Body(Body::from_meta(&attr.meta)?),
                "headers" => {
                    if attr.meta.require_path_only().is_err() {
                        let error = darling::Error::custom(
                            "There are no additional parameters for `#[headers]`.",
                        )
                        .with_span(&attr.meta);
                        return Err(error);
                    }
                    FieldSource::Headers
                }
                "path_params" => {
                    if attr.meta.require_path_only().is_err() {
                        let error = darling::Error::custom(
                            "There are no additional parameters for `#[path_params]`.",
                        )
                        .with_span(&attr.meta);
                        return Err(error);
                    }
                    FieldSource::PathParams
                }
                "query_params" => {
                    if attr.meta.require_path_only().is_err() {
                        let error = darling::Error::custom(
                            "There are no additional parameters for `#[query_params]`.",
                        )
                        .with_span(&attr.meta);
                        return Err(error);
                    }
                    FieldSource::QueryParams
                }
                _ => {
                    return Err(darling::Error::unknown_field_with_alts(
                        &ident,
                        &[
                            "path_param",
                            "query_param",
                            "header",
                            "body",
                            "path_params",
                            "query_params",
                            "headers",
                        ],
                    ));
                }
            };
            sources.push(source);
        }

        if sources.len() == 1 {
            let source = sources.remove(0);
            Ok(Self {
                ident: field.ident.clone().unwrap(),
                ty: field.ty.clone(),
                source,
            })
        } else if sources.is_empty() {
            let ident = field.ident.as_ref().unwrap();
            Err(darling::Error::custom(format!(
                "Field `{ident}` must specify a source.\nUse one of the following: \
                 #[path_param], #[query_param], #[header], #[body], \
                 #[query_params], #[path_params], or #[headers].",
            ))
            .with_span(&ident))
        } else {
            let ident = field.ident.as_ref().unwrap();
            let mut msg = format!(
                "There are multiple conflicting sources for field `{ident}`.\nUse only **one** of ",
            );
            for (i, source) in sources.iter().enumerate() {
                msg.push_str(&format!("#[{}]", source.source()));
                if i + 2 == sources.len() {
                    msg.push_str(" or ");
                } else if i + 1 != sources.len() {
                    msg.push_str(", ");
                }
            }
            msg.push_str(" as source.");

            let e = darling::Error::custom(msg)
                // TODO: I'd like to highlight the field name _and_ the attributes,
                //   but using `field` as the source produces a span that only includes
                //   the `#` token from the very first attribute.
                .with_span(ident);
            Err(e)
        }
    }
}

impl FieldSource {
    pub fn source(&self) -> &str {
        match self {
            FieldSource::PathParam(_) => "path_param",
            FieldSource::PathParams => "path_params",
            FieldSource::QueryParam(_) => "query_param",
            FieldSource::QueryParams => "query_params",
            FieldSource::Header(_) => "header",
            FieldSource::Headers => "headers",
            FieldSource::Body(_) => "body",
        }
    }
}

pub(super) fn derive_from_request(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match _derive_from_request(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.write_errors().into(),
    }
}

fn _derive_from_request(input: DeriveInput) -> Result<proc_macro2::TokenStream, darling::Error> {
    let input = FromRequestInput::from_derive_input(&input)?;
    reject_invalid_inputs(&input)?;

    let struct_ident = &input.ident;
    let fields = input
        .data
        .take_struct()
        // This should never panic, since we reject unsupported shapes earlier on, automatically,
        // via `darling`.
        .expect("`FromRequest` only supports structs with named fields.");

    let required_sources = RequiredInputs::compute(fields.iter());

    // Let codegen begin!
    let raw_path_params_ident = format_ident!("__path_params");
    let request_head_ident = format_ident!("__request_head");
    let body_ident = format_ident!("__body");
    let errors_ident = format_ident!("__errors");
    let intermediate_path_params_ident = format_ident!("__intermediate_path_params");
    let intermediate_query_params_ident = format_ident!("__intermediate_query_params");

    let input_parameters = {
        let mut params = Vec::with_capacity(3);
        if required_sources.raw_path_params {
            params.push(
                quote! { #raw_path_params_ident: &pavex::request::path::RawPathParams<'_, '_> },
            );
        }
        if required_sources.request_head {
            params.push(quote! { #request_head_ident: &pavex::request::RequestHead });
        }
        if required_sources.body {
            params.push(quote! { #body_ident: &pavex::request::body::BufferedBody });
        }
        params
    };

    let mut grouped_pp_fields = Vec::new();
    let mut grouped_qp_fields = Vec::new();
    let mut extractions = Vec::with_capacity(fields.len());
    for field in fields.iter() {
        let field_ident = &field.ident;
        let ty_span = field.ty.span();
        let extraction = match &field.source {
            FieldSource::QueryParam(..) => {
                grouped_qp_fields.push(field);
                continue;
            }
            FieldSource::PathParam(..) => {
                grouped_pp_fields.push(field);
                continue;
            }
            FieldSource::PathParams => {
                quote_spanned! {
                    ty_span => ::pavex::request::path::PathParams::extract(#raw_path_params_ident)
                }
            }
            FieldSource::QueryParams => {
                quote_spanned! {
                    ty_span => ::pavex::request::query::QueryParams::extract(#request_head_ident)
                }
            }
            FieldSource::Header(..) | FieldSource::Headers => todo!(),
            FieldSource::Body(body) => match body.format.trim().to_lowercase().as_str() {
                "json" => {
                    quote_spanned! {
                        ty_span => ::pavex::request::body::JsonBody::extract(#request_head_ident, #body_ident)
                    }
                }
                format => {
                    return Err(darling::Error::custom(format!(
                        "`{format}` isn't one of the supported formats for the `#[body(..)]` attribute in `#[derive(FromRequest)].`"
                    )).with_span(&field.ident));
                }
            },
        };
        extractions.push(quote_spanned! { ty_span =>
            let #field_ident = match #extraction {
                Ok(v) => Some(v.0),
                Err(e) => {
                    #errors_ident.push(e);
                    None
                },
            };
        });
    }

    // Collect all the keys that must be populated from on a specific path parameter.
    let pp_intermediate = {
        if grouped_pp_fields.is_empty() {
            None
        } else {
            let field_defs = grouped_pp_fields.iter().filter_map(|f| {
                let FieldSource::PathParam(param) = &f.source else {
                    return None;
                };
                let key = param.name.clone().unwrap_or_else(|| f.ident.to_string());
                let ident = format_ident!("{key}");
                let ty = &f.ty;
                let ty_span = ty.span();
                Some(quote_spanned! { ty_span => #ident: #ty })
            });
            let def = quote! {
                #[derive(::serde::Deserialize)]
                struct __RequiredPathParams {
                    #(#field_defs),*
                }

                let #intermediate_path_params_ident: Option<__RequiredPathParams> = match ::pavex::request::path::PathParams::extract(#raw_path_params_ident) {
                    Ok(params) => Some(params.0),
                    Err(e) => {
                        #errors_ident.push(e);
                        None
                    }
                };
            };
            Some(def)
        }
    };

    // Collect all the keys that must be populated from a specific query parameter.
    let qp_intermediate = {
        if grouped_qp_fields.is_empty() {
            None
        } else {
            let field_defs = grouped_qp_fields.iter().filter_map(|f| {
                let FieldSource::QueryParam(param) = &f.source else {
                    return None;
                };
                let key = param.name.clone().unwrap_or_else(|| f.ident.to_string());
                let ident = format_ident!("{key}");
                let ty = &f.ty;
                let ty_span = ty.span();
                Some(quote_spanned! { ty_span => #ident: #ty })
            });
            let def = quote! {
                #[derive(::serde::Deserialize)]
                struct __RequiredQueryParams {
                    #(#field_defs),*
                }

                let #intermediate_query_params_ident: Option<__RequiredQueryParams> = match ::pavex::request::query::QueryParams::extract(#request_head_ident) {
                    Ok(params) => Some(params.0),
                    Err(e) => {
                        #errors_ident.push(e);
                        None
                    }
                };
            };
            Some(def)
        }
    };

    let field_assignments = fields.iter().map(|field| {
        let field_ident = &field.ident;
        let ty_span = field.ty.span();
        let unwrapping = match &field.source {
            FieldSource::QueryParam(qp) => {
                let name = if let Some(name) = &qp.name {
                    &format_ident!("{name}")
                } else {
                    field_ident
                };
                quote_spanned! {
                    ty_span => #intermediate_query_params_ident.#name
                }
            }
            FieldSource::PathParam(pp) => {
                let name = if let Some(name) = &pp.name {
                    &format_ident!("{name}")
                } else {
                    field_ident
                };
                quote_spanned! {
                    ty_span => #intermediate_path_params_ident.#name
                }
            }
            FieldSource::Body(..) | FieldSource::QueryParams | FieldSource::PathParams => {
                quote_spanned! {
                    ty_span => #field_ident.unwrap()
                }
            }
            FieldSource::Header(..) | FieldSource::Headers => todo!(),
        };
        quote_spanned! {
            ty_span => #field_ident: #unwrapping
        }
    });
    let path_params_intermediate_unwrap = pp_intermediate.is_some().then(|| {
        quote! {
            let #intermediate_path_params_ident = #intermediate_path_params_ident.unwrap();
        }
    });
    let query_params_intermediate_unwrap = qp_intermediate.is_some().then(|| {
        quote! {
            let #intermediate_query_params_ident = #intermediate_query_params_ident.unwrap();
        }
    });

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let tokens = quote! {
        #[::pavex::methods]
        impl #impl_generics #struct_ident #ty_generics #where_clause {
            #[request_scoped]
            pub fn from_request(#(#input_parameters),*) -> Result<Self, ::pavex::request::errors::FromRequestErrors> {
                let mut #errors_ident = ::pavex::request::errors::FromRequestErrors::new();
                // Perform all fallible operations first, collecting errors as they occur.
                #pp_intermediate
                #qp_intermediate
                #(#extractions)*

                // If anything failed, return the combined errors.
                if !#errors_ident.is_empty() {
                    return Err(#errors_ident)
                }

                // Nothing failed, we can safely unwrap each field.
                #path_params_intermediate_unwrap
                #query_params_intermediate_unwrap
                Ok(Self {
                    #(
                        #field_assignments
                    ),*
                })
            }
        }
    };

    Ok(tokens)
}

fn reject_invalid_inputs(input: &FromRequestInput) -> Result<(), darling::Error> {
    let struct_ident = &input.ident;
    // Reject structs with generic type parameters.
    if let Some(generic) = input.generics.type_params().next() {
        return Err(darling::Error::custom(format!(
            "`#[derive(FromRequest)]` can't be applied to structs with generic type parameters, such as `{struct_ident}`.\n\n\
            help: Consider using concrete types instead. Alternatively, define your own constructor for `{struct_ident}`.",
        ))
        .with_span(generic));
    }
    // We plan to allow lifetimes in the future, but for the time being we're punting on them
    // to keep implementation complexity down.
    if let Some(lifetime) = input.generics.lifetimes().next() {
        return Err(darling::Error::custom(format!(
            "`#[derive(FromRequest)]` can't be applied to structs with generic lifetimes, such as `{struct_ident}`.\n\n\
            help: Define your own request-scoped constructor for `{struct_ident}`.",
        ))
        .with_span(lifetime));
    }

    // Require the struct to be public
    if !matches!(input.vis, syn::Visibility::Public(_)) {
        let error = match &input.vis {
            syn::Visibility::Public(_) => unreachable!(),
            syn::Visibility::Restricted(res) => darling::Error::custom(format!(
                "`{struct_ident}` must be `pub`, with no additional visibility restrictions.\n\n\
                help: Remove the highlighted visibility modifier.",
            ))
            .with_span(&res.paren_token.span.join()),
            syn::Visibility::Inherited => darling::Error::custom(format!(
                "`{struct_ident}` must be `pub`.\n\n\
                    help: Add `pub` in front of the `struct` keyword.",
            ))
            .with_span(&input.ident),
        };
        return Err(error);
    }
    Ok(())
}

#[derive(Default, Copy, Clone)]
/// The input parameters taken by the generated `from_request` constructor.
struct RequiredInputs {
    raw_path_params: bool,
    request_head: bool,
    body: bool,
}

impl RequiredInputs {
    fn compute<'a>(fields: impl Iterator<Item = &'a ParsedField>) -> Self {
        fields.fold(RequiredInputs::default(), |acc, field| RequiredInputs {
            raw_path_params: acc.raw_path_params
                || matches!(
                    field.source,
                    FieldSource::PathParams | FieldSource::PathParam(..)
                ),
            request_head: acc.request_head
                || matches!(
                    field.source,
                    FieldSource::QueryParams
                            | FieldSource::QueryParam(..)
                            | FieldSource::Headers
                            | FieldSource::Header(..)
                            // We need the head to check the Content-Type header.
                            | FieldSource::Body(..)
                ),
            body: acc.body || matches!(field.source, FieldSource::Body(..)),
        })
    }
}
