use std::collections::HashMap;

use bimap::BiHashMap;
use guppy::PackageId;
use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use petgraph::Direction;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};

use crate::language::{Callable, ResolvedType};
use crate::web::dependency_graph::DependencyGraphNode;

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

// Generate a sequence of unique variable names.
#[derive(Default)]
pub(crate) struct VariableNameGenerator {
    cursor: u32,
}

impl VariableNameGenerator {
    pub fn generate(&mut self) -> syn::Ident {
        let ident = format_ident!("v{}", self.cursor);
        self.cursor += 1;
        ident
    }
}

pub(crate) fn codegen_call_block(
    call_graph: &StableDiGraph<DependencyGraphNode, ()>,
    callable: &Callable,
    node_index: NodeIndex,
    blocks: &mut HashMap<NodeIndex, Fragment>,
    variable_generator: &mut VariableNameGenerator,
    package_id2name: &BiHashMap<&PackageId, String>,
) -> Result<Fragment, anyhow::Error> {
    let dependencies = call_graph.neighbors_directed(node_index, Direction::Incoming);
    let mut block = quote! {};
    let mut dependency_bindings: BiHashMap<ResolvedType, syn::Ident> = BiHashMap::new();
    for dependency_index in dependencies {
        let fragment = &blocks[&dependency_index];
        let dependency_type = match &call_graph[dependency_index] {
            DependencyGraphNode::Type(t) => t,
            DependencyGraphNode::Compute(_) => unreachable!(),
        };
        let mut to_be_removed = false;
        match fragment {
            Fragment::VariableReference(v) => {
                dependency_bindings.insert(dependency_type.to_owned(), v.to_owned());
            }
            Fragment::Block(_) | Fragment::Statement(_) => {
                let parameter_name = variable_generator.generate();
                dependency_bindings.insert(dependency_type.to_owned(), parameter_name.to_owned());
                to_be_removed = true;
                block = quote! {
                    #block
                    let #parameter_name = #fragment;
                }
            }
        }
        if to_be_removed {
            // It won't be needed in the future
            blocks.remove(&dependency_index);
        }
    }
    let constructor_invocation = codegen_call(callable, &dependency_bindings, package_id2name)?;
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
    variable_bindings: &BiHashMap<ResolvedType, syn::Ident>,
    package_id2name: &BiHashMap<&PackageId, String>,
) -> Result<syn::ExprCall, anyhow::Error> {
    let callable_path: syn::ExprPath =
        syn::parse_str(&callable.path.render_path(package_id2name)).unwrap();
    let parameters = callable
        .inputs
        .iter()
        .map(|i| variable_bindings.get_by_left(i).unwrap());
    Ok(syn::parse2(quote! {
        #callable_path(#(#parameters),*)
    })
    .unwrap())
}
