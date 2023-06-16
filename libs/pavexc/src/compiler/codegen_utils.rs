use ahash::{HashMap, HashMapExt};
use bimap::BiHashMap;
use guppy::PackageId;
use petgraph::stable_graph::NodeIndex;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};

use crate::compiler::analyses::call_graph::CallGraphEdgeMetadata;
use crate::language::{Callable, InvocationStyle, ResolvedType, TypeReference};

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

pub(crate) fn codegen_call_block<I>(
    dependencies: I,
    callable: &Callable,
    blocks: &mut HashMap<NodeIndex, Fragment>,
    variable_generator: &mut VariableNameGenerator,
    package_id2name: &BiHashMap<PackageId, String>,
) -> Result<Fragment, anyhow::Error>
where
    I: Iterator<Item = (NodeIndex, ResolvedType, CallGraphEdgeMetadata)>,
{
    let mut block = quote! {};
    let mut dependency_bindings: HashMap<ResolvedType, Box<dyn ToTokens>> = HashMap::new();
    for (dependency_index, dependency_type, consumption_mode) in dependencies {
        let type_ = match consumption_mode {
            CallGraphEdgeMetadata::Move => dependency_type.to_owned(),
            CallGraphEdgeMetadata::SharedBorrow => ResolvedType::Reference(TypeReference {
                is_mutable: false,
                is_static: false,
                inner: Box::new(dependency_type.to_owned()),
            }),
        };
        let fragment = &blocks[&dependency_index];
        let mut to_be_removed = false;
        let tokens = match fragment {
            Fragment::VariableReference(v) => match consumption_mode {
                CallGraphEdgeMetadata::Move => Box::new(quote! { #v }),
                CallGraphEdgeMetadata::SharedBorrow => Box::new(quote! { &#v }),
            },
            Fragment::Block(_) | Fragment::Statement(_) => {
                let parameter_name = variable_generator.generate();
                to_be_removed = true;
                block = quote! {
                    #block
                    let #parameter_name = #fragment;
                };
                match consumption_mode {
                    CallGraphEdgeMetadata::Move => Box::new(quote! { #parameter_name }),
                    CallGraphEdgeMetadata::SharedBorrow => Box::new(quote! { &#parameter_name }),
                }
            }
        };
        dependency_bindings.insert(type_, tokens);
        if to_be_removed {
            // It won't be needed in the future
            blocks.remove(&dependency_index);
        }
    }
    let constructor_invocation = codegen_call(callable, &dependency_bindings, package_id2name);
    let block: syn::Block = syn::parse2(quote! {
        {
            #block
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
            let parameters = callable.inputs.iter().map(|i| &variable_bindings[i]);
            quote! {
                #callable_path(#(#parameters),*)
            }
        }
        InvocationStyle::StructLiteral { field_names } => {
            let fields = field_names.iter().map(|(field_name, field_type)| {
                let field_name = format_ident!("{}", field_name);
                let binding = &variable_bindings[field_type];
                quote! {
                    #field_name: #binding
                }
            });
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
