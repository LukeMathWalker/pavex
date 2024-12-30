use ahash::{HashMap, HashMapExt};
use bimap::BiHashMap;
use guppy::PackageId;
use petgraph::stable_graph::NodeIndex;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};

use crate::compiler::analyses::call_graph::CallGraphEdgeMetadata;
use crate::language::{Callable, InvocationStyle, Lifetime, ResolvedType, TypeReference};

#[derive(Debug, Clone)]
pub(crate) enum Fragment {
    VariableReference(syn::Ident),
    Statement(Box<syn::Stmt>),
    Block(syn::Block),
}

impl ToTokens for Fragment {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Fragment::VariableReference(v) => v.to_tokens(tokens),
            Fragment::Statement(s) => s.to_tokens(tokens),
            Fragment::Block(b) => b.to_tokens(tokens),
        }
    }
}

/// A stateful generator of unique variable names.
#[derive(Default, Clone)]
pub(crate) struct VariableNameGenerator {
    cursor: u32,
}

impl VariableNameGenerator {
    /// Create a new variable name generator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate a new variable name.
    pub fn generate(&mut self) -> syn::Ident {
        let ident = format_ident!("v{}", self.cursor);
        self.cursor += 1;
        ident
    }
}

pub(crate) fn codegen_call_block<I, J>(
    dependencies: I,
    happen_befores: J,
    callable: &Callable,
    blocks: &mut HashMap<NodeIndex, Fragment>,
    variable_generator: &mut VariableNameGenerator,
    package_id2name: &BiHashMap<PackageId, String>,
) -> Result<Fragment, anyhow::Error>
where
    I: Iterator<Item = (NodeIndex, ResolvedType, CallGraphEdgeMetadata)>,
    J: Iterator<Item = NodeIndex>,
{
    let mut before_block = quote! {};
    for happen_before in happen_befores {
        let fragment = &blocks[&happen_before];
        before_block = quote! {
            #before_block
            #fragment;
        };
    }

    let mut dependency_bindings: HashMap<ResolvedType, Box<dyn ToTokens>> = HashMap::new();
    let mut dependency_blocks = Vec::new();
    for (dependency_index, dependency_type, consumption_mode) in dependencies {
        let type_ = match consumption_mode {
            CallGraphEdgeMetadata::Move => dependency_type.to_owned(),
            CallGraphEdgeMetadata::SharedBorrow => ResolvedType::Reference(TypeReference {
                is_mutable: false,
                lifetime: Lifetime::Elided,
                inner: Box::new(dependency_type.to_owned()),
            }),
            CallGraphEdgeMetadata::ExclusiveBorrow => ResolvedType::Reference(TypeReference {
                is_mutable: true,
                lifetime: Lifetime::Elided,
                inner: Box::new(dependency_type.to_owned()),
            }),
            CallGraphEdgeMetadata::HappensBefore => {
                unreachable!()
            }
        };
        let Some(fragment) = &blocks.get(&dependency_index) else {
            panic!("Failed to find the code fragment for {dependency_index:?}, the node that builds `{dependency_type:?}`");
        };
        let mut to_be_removed = false;
        let tokens = match fragment {
            Fragment::VariableReference(v) => match consumption_mode {
                CallGraphEdgeMetadata::Move => Box::new(quote! { #v }),
                CallGraphEdgeMetadata::SharedBorrow => Box::new(quote! { &#v }),
                CallGraphEdgeMetadata::ExclusiveBorrow => Box::new(quote! { &mut #v }),
                CallGraphEdgeMetadata::HappensBefore => unreachable!(),
            },
            Fragment::Block(_) | Fragment::Statement(_) => {
                let parameter_name = variable_generator.generate();
                to_be_removed = true;
                dependency_blocks.push(quote! {
                    let #parameter_name = #fragment;
                });
                match consumption_mode {
                    CallGraphEdgeMetadata::Move => Box::new(quote! { #parameter_name }),
                    CallGraphEdgeMetadata::SharedBorrow => Box::new(quote! { &#parameter_name }),
                    CallGraphEdgeMetadata::ExclusiveBorrow => {
                        Box::new(quote! { &mut #parameter_name })
                    }
                    CallGraphEdgeMetadata::HappensBefore => {
                        unreachable!()
                    }
                }
            }
        };

        // We also register the reference as a shared reference
        // since we rely on the compiler's deref coercion to convert
        // `&mut T` to `&T` when needed.
        if let ResolvedType::Reference(TypeReference {
            is_mutable: true,
            lifetime,
            inner,
        }) = &type_
        {
            dependency_bindings.insert(
                ResolvedType::Reference(TypeReference {
                    is_mutable: false,
                    lifetime: lifetime.to_owned(),
                    inner: inner.to_owned(),
                }),
                tokens.clone(),
            );
        }

        dependency_bindings.insert(type_, tokens);

        if to_be_removed {
            // It won't be needed in the future
            blocks.remove(&dependency_index);
        }
    }
    let constructor_invocation = codegen_call(callable, &dependency_bindings, package_id2name);
    let block: syn::Block = syn::parse2(quote! {
        {
            #before_block
            #(#dependency_blocks)*
            #constructor_invocation
        }
    })
    .unwrap();
    if block.stmts.len() == 1 {
        Ok(Fragment::Statement(Box::new(
            block.stmts.first().unwrap().to_owned(),
        )))
    } else {
        Ok(Fragment::Block(block))
    }
}

pub(crate) fn codegen_call(
    callable: &Callable,
    variable_bindings: &HashMap<ResolvedType, Box<dyn ToTokens>>,
    package_id2name: &BiHashMap<PackageId, String>,
) -> TokenStream {
    let callable_path: syn::ExprPath = {
        let mut buffer = String::new();
        callable.path.render_path(package_id2name, &mut buffer);
        syn::parse_str(&buffer).unwrap()
    };
    let mut invocation = match &callable.invocation_style {
        InvocationStyle::FunctionCall => {
            let parameters = callable.inputs.iter().map(|i| {
                match variable_bindings.get(i) {
                    Some(tokens) => tokens,
                    None => {
                        use std::fmt::Write as _;

                        let mut msg = String::new();
                        for ty in variable_bindings.keys() {
                            let _ = writeln!(&mut msg, "- {ty:?}`");
                        }
                        panic!(
                            "There is no variable with type {i:?} in scope.\nTypes of bound variables:\n{msg}",
                        )
                    }
                }
            });
            quote! {
                #callable_path(#(#parameters),*)
            }
        }
        InvocationStyle::StructLiteral {
            field_names,
            extra_field2default_value,
        } => {
            let fields = field_names
                .iter()
                .map(|(field_name, field_type)| {
                    let field_name = format_ident!("{}", field_name);
                    let binding = match variable_bindings.get(field_type) {
                        Some(tokens) => tokens,
                        None => {
                            use std::fmt::Write as _;

                            let mut msg = String::new();
                            for ty in variable_bindings.keys() {
                                let _ = writeln!(&mut msg, "- {ty:?}`");
                            }
                            panic!(
                                "There is no variable with type {field_type:?} in scope.\nTypes of bound variables:\n{msg}",
                            )
                        }
                    };
                    quote! {
                        #field_name: #binding
                    }
                })
                .chain(
                    extra_field2default_value
                        .iter()
                        .map(|(field_name, default_value)| {
                            let field_name = format_ident!("{}", field_name);
                            let default_value = format_ident!("{}", default_value);
                            quote! {
                                #field_name: #default_value
                            }
                        }),
                );
            quote! {
                #callable_path {
                    #(#fields),*
                }
            }
        }
    };
    if callable.is_async {
        invocation = quote! { #invocation.await };
    }
    invocation
}
