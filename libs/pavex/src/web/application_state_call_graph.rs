use std::collections::HashMap;

use bimap::BiHashMap;
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};
use petgraph::prelude::StableDiGraph;
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::{DfsPostOrder, Reversed};
use petgraph::Direction;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{ExprStruct, ItemFn};

use pavex_builder::Lifecycle;

use crate::language::{Callable, ResolvedPath, ResolvedPathSegment, ResolvedType};
use crate::web::app::GENERATED_APP_PACKAGE_ID;
use crate::web::codegen_utils::{codegen_call_block, Fragment, VariableNameGenerator};
use crate::web::constructors::Constructor;
use crate::web::dependency_graph::{CallableDependencyGraph, DependencyGraphNode};

#[derive(Debug)]
pub(crate) struct ApplicationStateCallGraph {
    pub(crate) call_graph: StableDiGraph<DependencyGraphNode, ()>,
    pub(crate) application_state_init_node_index: NodeIndex,
    pub(crate) lifecycles: HashMap<ResolvedType, Lifecycle>,
    pub(crate) constructors: IndexMap<ResolvedType, Constructor>,
    pub(crate) input_parameter_types: IndexSet<ResolvedType>,
    pub(crate) runtime_singleton_bindings: BiHashMap<Ident, ResolvedType>,
}

impl ApplicationStateCallGraph {
    #[tracing::instrument(name = "compute_application_state_call_graph", skip_all)]
    pub(crate) fn new(
        runtime_singleton_bindings: BiHashMap<Ident, ResolvedType>,
        lifecycles: HashMap<ResolvedType, Lifecycle>,
        constructors: IndexMap<ResolvedType, Constructor>,
    ) -> Self {
        // We build a "mock" callable that has the right inputs in order to drive the machinery
        // that builds the dependency graph.
        let application_state_constructor = Callable {
            is_async: false,
            output: ResolvedType {
                package_id: PackageId::new(GENERATED_APP_PACKAGE_ID),
                base_type: vec!["crate".into(), "ApplicationState".into()],
                generic_arguments: vec![],
                is_shared_reference: false,
            },
            path: ResolvedPath {
                segments: vec![
                    ResolvedPathSegment {
                        ident: "crate".into(),
                        generic_arguments: vec![],
                    },
                    ResolvedPathSegment {
                        ident: "ApplicationState".into(),
                        generic_arguments: vec![],
                    },
                ],
                package_id: PackageId::new(GENERATED_APP_PACKAGE_ID),
            },
            inputs: runtime_singleton_bindings.right_values().cloned().collect(),
        };
        let CallableDependencyGraph {
            dependency_graph,
            callable_node_index,
        } = CallableDependencyGraph::new(application_state_constructor, &constructors);

        // Vec<(index in dependency graph, parent index in call graph)>
        let mut nodes_to_be_visited = vec![(callable_node_index, None)];
        let mut singleton_or_longer_indexes = HashMap::<u32, NodeIndex>::new();
        let mut call_graph = StableDiGraph::new();
        while let Some((dep_node_index, call_parent_node_index)) = nodes_to_be_visited.pop() {
            let node = &dependency_graph[dep_node_index];
            // Determine how many times the constructor for this type should be invoked in the call graph.
            // If we are dealing with a singleton, we need to make sure it's invoked only once.
            // Transient components, instead, appear as many times as they are used in the call graph.
            // We treat compute nodes as singletons as well.
            let call_node_index = match node {
                DependencyGraphNode::Compute(_) => {
                    let index = call_graph.add_node(node.to_owned());
                    singleton_or_longer_indexes.insert(dep_node_index, index);
                    index
                }
                DependencyGraphNode::Type(t) => {
                    let lifecycle = lifecycles.get(t).cloned().unwrap_or(Lifecycle::Singleton);
                    match lifecycle {
                        Lifecycle::RequestScoped => {
                            panic!("Singletons should not depend on types with a request-scoped lifecycle.")
                        }
                        Lifecycle::Singleton => singleton_or_longer_indexes
                            .get(&dep_node_index)
                            .cloned()
                            .unwrap_or_else(|| {
                                let index = call_graph.add_node(node.to_owned());
                                singleton_or_longer_indexes.insert(dep_node_index, index);
                                index
                            }),
                        // TODO: determine if it's OK for a singleton to depend on a transient type.
                        // It might require distinguishing between runtime and build-time transient types.
                        Lifecycle::Transient => call_graph.add_node(node.to_owned()),
                    }
                }
            };
            if let Some(call_parent_node_index) = call_parent_node_index {
                call_graph.add_edge(call_node_index, call_parent_node_index, ());
            }

            let dependencies_node_indexes = dependency_graph
                .graph
                .neighbors_directed(dep_node_index, Direction::Incoming);
            for dependency_node_index in dependencies_node_indexes {
                nodes_to_be_visited.push((dependency_node_index, Some(call_node_index)));
            }
        }
        let input_parameter_types = required_inputs(&call_graph, &constructors);
        Self {
            call_graph,
            application_state_init_node_index: singleton_or_longer_indexes[&callable_node_index],
            lifecycles,
            constructors,
            input_parameter_types,
            runtime_singleton_bindings,
        }
    }

    pub(crate) fn codegen<'a>(
        &self,
        package_id2name: &BiHashMap<&'a PackageId, String>,
    ) -> Result<ItemFn, anyhow::Error> {
        let Self {
            call_graph,
            application_state_init_node_index: handler_node_index,
            lifecycles,
            constructors,
            input_parameter_types,
            runtime_singleton_bindings,
        } = &self;
        let mut dfs = DfsPostOrder::new(Reversed(call_graph), *handler_node_index);

        let mut parameter_bindings: HashMap<ResolvedType, Ident> = HashMap::new();
        let mut variable_generator = VariableNameGenerator::default();

        let mut singleton_constructors = HashMap::<NodeIndex, TokenStream>::new();
        let mut blocks = HashMap::<NodeIndex, Fragment>::new();

        while let Some(node_index) = dfs.next(Reversed(call_graph)) {
            let node = &call_graph[node_index];
            match node {
                DependencyGraphNode::Type(t) => {
                    let lifecycle = lifecycles.get(t).cloned().unwrap_or(Lifecycle::Singleton);
                    match lifecycle {
                        Lifecycle::Singleton => {
                            let parameter_name = variable_generator.generate();
                            match constructors.get(t) {
                                None => {
                                    parameter_bindings.insert(t.to_owned(), parameter_name.clone());
                                }
                                Some(constructor) => match constructor {
                                    Constructor::Callable(callable) => {
                                        let block = codegen_call_block(
                                            call_graph,
                                            callable,
                                            node_index,
                                            &mut blocks,
                                            &mut variable_generator,
                                            package_id2name,
                                        )?;
                                        let block = quote! {
                                            let #parameter_name = #block;
                                        };
                                        singleton_constructors.insert(node_index, block);
                                    }
                                    Constructor::BorrowSharedReference(_) => unreachable!(),
                                },
                            };
                            blocks.insert(node_index, Fragment::VariableReference(parameter_name));
                        }
                        Lifecycle::Transient => {
                            let constructor = &constructors[t];
                            match constructor {
                                Constructor::Callable(callable) => {
                                    let block = codegen_call_block(
                                        call_graph,
                                        callable,
                                        node_index,
                                        &mut blocks,
                                        &mut variable_generator,
                                        package_id2name,
                                    )?;
                                    blocks.insert(node_index, block);
                                }
                                Constructor::BorrowSharedReference(shared_reference) => {
                                    let variable_name =
                                        parameter_bindings.get(&shared_reference.input).unwrap();
                                    blocks.insert(
                                        node_index,
                                        Fragment::BorrowSharedReference(variable_name.to_owned()),
                                    );
                                }
                            }
                        }
                        Lifecycle::RequestScoped => unreachable!(),
                    }
                }
                DependencyGraphNode::Compute(callable) => {
                    let block = codegen_struct_init_block(
                        call_graph,
                        callable,
                        runtime_singleton_bindings,
                        node_index,
                        &mut blocks,
                        &mut variable_generator,
                        package_id2name,
                    )?;
                    blocks.insert(node_index, block);
                }
            }
        }

        let handler = match &call_graph[*handler_node_index] {
            DependencyGraphNode::Compute(c) => c,
            DependencyGraphNode::Type(_) => unreachable!(),
        };
        let code = match blocks.remove(handler_node_index) {
            None => unreachable!(),
            Some(b) => {
                let inputs = input_parameter_types.iter().map(|type_| {
                    let variable_name = &parameter_bindings[type_];
                    let variable_type = type_.syn_type(package_id2name);
                    quote! { #variable_name: #variable_type }
                });
                let output_type = handler.output.syn_type(package_id2name);
                let singleton_constructors = singleton_constructors.values();
                let b = match b {
                    Fragment::BorrowSharedReference(_) | Fragment::VariableReference(_) => {
                        unreachable!()
                    }
                    Fragment::Block(b) => {
                        let stmts = b.stmts.iter();
                        quote! {
                            #(#stmts)*
                        }
                    }
                    Fragment::Statement(s) => s.to_token_stream(),
                };
                syn::parse2(quote! {
                    pub fn build_application_state(#(#inputs),*) -> #output_type {
                        #(#singleton_constructors)*
                        #b
                    }
                })
                .unwrap()
            }
        };
        Ok(code)
    }

    pub fn dot(&self, package_ids2names: &BiHashMap<&'_ PackageId, String>) -> String {
        let config = [
            petgraph::dot::Config::EdgeNoLabel,
            petgraph::dot::Config::NodeNoLabel,
        ];
        format!(
            "{:?}",
            petgraph::dot::Dot::with_attr_getters(
                &self.call_graph,
                &config,
                &|_, _| "".to_string(),
                &|_, (_, node)| {
                    match node {
                        DependencyGraphNode::Compute(c) => {
                            format!("label = \"{}\"", c.render_signature(package_ids2names))
                        }
                        DependencyGraphNode::Type(t) => {
                            format!("label = \"{}\"", t.render_type(package_ids2names))
                        }
                    }
                },
            )
        )
        .replace("digraph", "digraph app_state")
    }
}

pub(crate) fn codegen_struct_init_block(
    call_graph: &StableDiGraph<DependencyGraphNode, ()>,
    callable: &Callable,
    struct_fields: &BiHashMap<Ident, ResolvedType>,
    node_index: NodeIndex,
    blocks: &mut HashMap<NodeIndex, Fragment>,
    variable_generator: &mut VariableNameGenerator,
    package_id2name: &BiHashMap<&PackageId, String>,
) -> Result<Fragment, anyhow::Error> {
    let dependencies = call_graph.neighbors_directed(node_index, Direction::Incoming);
    let mut block = quote! {};
    let mut dependency_bindings = HashMap::<ResolvedType, Box<dyn ToTokens>>::new();
    for dependency_index in dependencies {
        let fragment = &blocks[&dependency_index];
        let dependency_type = match &call_graph[dependency_index] {
            DependencyGraphNode::Type(t) => t,
            DependencyGraphNode::Compute(_) => unreachable!(),
        };
        let mut to_be_removed = false;
        match fragment {
            Fragment::VariableReference(v) => {
                dependency_bindings.insert(dependency_type.to_owned(), Box::new(v.to_owned()));
            }
            Fragment::Block(_) | Fragment::Statement(_) => {
                let parameter_name = variable_generator.generate();
                dependency_bindings.insert(
                    dependency_type.to_owned(),
                    Box::new(parameter_name.to_owned()),
                );
                to_be_removed = true;
                block = quote! {
                    #block
                    let #parameter_name = #fragment;
                }
            }
            Fragment::BorrowSharedReference(v) => {
                dependency_bindings.insert(dependency_type.to_owned(), Box::new(quote! { &#v }));
            }
        }
        if to_be_removed {
            // It won't be needed in the future
            blocks.remove(&dependency_index);
        }
    }
    let constructor_invocation = codegen_struct_init(
        &callable.path,
        struct_fields,
        &dependency_bindings,
        package_id2name,
    )?;
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

pub(crate) fn codegen_struct_init(
    struct_path: &ResolvedPath,
    struct_fields: &BiHashMap<Ident, ResolvedType>,
    field_init_values: &HashMap<ResolvedType, Box<dyn ToTokens>>,
    id2name: &BiHashMap<&PackageId, String>,
) -> Result<ExprStruct, anyhow::Error> {
    let struct_path: syn::ExprPath = syn::parse_str(&struct_path.render_path(id2name)).unwrap();
    let fields = struct_fields.iter().map(|(field_name, field_type)| {
        let binding = &field_init_values[field_type];
        quote! {
            #field_name: #binding
        }
    });
    Ok(syn::parse2(quote! {
        #struct_path {
            #(#fields),*
        }
    })
    .unwrap())
}

/// Return the set of types that must be provided as input to build the application state.
///
/// In other words, return the set of types that do not have a registered constructor.
///
/// We return a `IndexSet` instead of a `HashSet` because we want a consistent ordering for the input
/// parameters - it will be used in other parts of the crate to provide instances of those types
/// in the expected order.
fn required_inputs(
    call_graph: &StableDiGraph<DependencyGraphNode, ()>,
    constructors: &IndexMap<ResolvedType, Constructor>,
) -> IndexSet<ResolvedType> {
    call_graph
        .node_weights()
        .filter_map(|node| {
            if let DependencyGraphNode::Type(type_) = node {
                if constructors.get(type_).is_none() {
                    return Some(type_);
                }
            }
            None
        })
        .cloned()
        .collect()
}
