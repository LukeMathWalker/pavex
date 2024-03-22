use std::collections::BTreeMap;

use ahash::{HashMap, HashMapExt};
use guppy::graph::PackageGraph;
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use quote::quote;

use pavex_bp_schema::{CloningStrategy, Lifecycle};

use crate::compiler::analyses::call_graph::{
    request_scoped_call_graph, request_scoped_ordered_call_graph, CallGraphNode,
    InputParameterSource, OrderedCallGraph, RawCallGraph, RawCallGraphExt,
};
use crate::compiler::analyses::components::component::Component;
use crate::compiler::analyses::components::HydratedComponent;
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::compiler::analyses::framework_items::FrameworkItemDb;
use crate::compiler::app::GENERATED_APP_PACKAGE_ID;
use crate::compiler::computation::Computation;
use crate::compiler::utils::LifetimeGenerator;
use crate::language::{
    Callable, GenericArgument, GenericLifetimeParameter, InvocationStyle, Lifetime, PathType,
    ResolvedType, TypeReference,
};
use crate::rustdoc::CrateCollection;

/// A request handler pipeline is the combination of a root compute node (i.e. the request handler)
/// and an ordered sequence of wrapping middlewares ahead of it, feeding into each other.
pub(crate) struct RequestHandlerPipeline {
    /// The name of the module where the pipeline is defined.
    pub(crate) module_name: String,
    pub(crate) id2call_graph: IndexMap<ComponentId, OrderedCallGraph>,
    pub(crate) stages: Vec<Stage>,
}

#[derive(Clone, Debug)]
pub struct Stage {
    /// The name of the function that represents this stage.
    pub(crate) name: String,
    /// Either a wrapping middleware or a request handler.
    pub(crate) wrapping_id: ComponentId,
    /// The input parameters for this stage of the pipeline (excluding the response type).
    pub(crate) input_parameters: InputParameters,
    /// Only set if `component_id` is a wrapping middleware.
    pub(crate) next_state: Option<NextState>,
    /// Post-processing middlewares to be invoked after `computation_id` has completed,
    /// but before returning to the caller.
    pub(crate) post_processing_ids: Vec<ComponentId>,
}

/// The "state" for `Next<T>` is the concrete type for `T` used in a specific middleware invocation.
///
/// It is computed on a per-pipeline and per-middleware basis, in order to pass down the
/// strict minimum of request-scoped and singleton components that are needed by the downstream
/// stages of the pipeline.
#[derive(Clone, Debug)]
pub(crate) struct NextState {
    pub(crate) type_: PathType,
    /// The state is always a struct.
    /// This map contains the bindings for the fields of the struct: the field name and the type
    /// of the field.
    pub(crate) field_bindings: Bindings,
}

impl RequestHandlerPipeline {
    /// Build a [`RequestHandlerPipeline`] for the request handler with the provided [`ComponentId`].
    pub(crate) fn new(
        handler_id: ComponentId,
        module_name: String,
        computation_db: &mut ComputationDb,
        component_db: &mut ComponentDb,
        constructible_db: &mut ConstructibleDb,
        framework_item_db: &FrameworkItemDb,
        package_graph: &PackageGraph,
        krate_collection: &CrateCollection,
        diagnostics: &mut Vec<miette::Error>,
    ) -> Result<Self, ()> {
        let error_observer_ids = component_db.error_observers(handler_id).unwrap().to_owned();

        // Step 1: Determine the sequence of middlewares that the request handler is wrapped in.
        let ordered_by_registration = {
            let mut v = component_db
                .middleware_chain(handler_id)
                .unwrap()
                .to_owned();
            v.push(handler_id);
            v
        };

        let stage_names = {
            let mut stage_names = vec![];
            let mut current_stage_id = 0;
            for middleware_id in &ordered_by_registration {
                let middleware = component_db.hydrated_component(*middleware_id, computation_db);
                match middleware {
                    HydratedComponent::RequestHandler(_)
                    | HydratedComponent::WrappingMiddleware(_) => {
                        if current_stage_id == 0 {
                            stage_names.push("entrypoint".to_string());
                        } else {
                            stage_names.push(format!("stage_{current_stage_id}"));
                        }
                        current_stage_id += 1;
                    }
                    _ => {}
                };
            }
            stage_names
        };

        let grouped_by_stage = {
            // Partition middlewares into groups, where each group contains 1 wrapping middleware
            // and all the post-processing middlewares (+handlers) that are invoked after the next
            // wrapping middleware has completed but before that wrapping middleware completes.
            // Within each group, we sort the middlewares by their invocation order.
            let first = component_db
                .hydrated_component(*ordered_by_registration.first().unwrap(), computation_db);
            assert!(
                matches!(first, HydratedComponent::WrappingMiddleware(_)),
                "First component should be a wrapping middleware, but it's a {:?}",
                first
            );
            let mut groups = vec![];
            for id in ordered_by_registration.iter() {
                let component = component_db.hydrated_component(*id, computation_db);
                if let HydratedComponent::WrappingMiddleware(_) = component {
                    groups.push(vec![*id]);
                } else {
                    groups.last_mut().unwrap().push(*id);
                }
            }
            let mut grouped_by_stage = vec![];
            for group in groups {
                grouped_by_stage.extend(
                    group
                        .iter()
                        .filter(|id| {
                            !component_db
                                .hydrated_component(**id, computation_db)
                                .is_post_processing_middleware()
                        })
                        .cloned()
                        .chain(
                            group
                                .iter()
                                .filter(|id| {
                                    component_db
                                        .hydrated_component(**id, computation_db)
                                        .is_post_processing_middleware()
                                })
                                .cloned(),
                        ),
                );
            }
            grouped_by_stage
        };

        // Step 2: For each middleware, build an ordered call graph.
        // We associate each request-scoped component with the set of middlewares that need it.
        let mut request_scoped_id2users: IndexMap<ComponentId, IndexSet<ComponentId>> =
            IndexMap::new();
        let mut id2call_graphs = HashMap::with_capacity(grouped_by_stage.len());

        for middleware_id in grouped_by_stage.iter() {
            let call_graph = request_scoped_call_graph(
                *middleware_id,
                &IndexSet::new(),
                &error_observer_ids,
                computation_db,
                component_db,
                constructible_db,
                diagnostics,
            )?;

            // Add all request-scoped components initialized by this component to the set.
            for request_scoped_id in
                extract_request_scoped_compute_nodes(&call_graph.call_graph, component_db)
            {
                request_scoped_id2users
                    .entry(request_scoped_id)
                    .or_default()
                    .insert(*middleware_id);
            }

            id2call_graphs.insert(*middleware_id, call_graph);
        }

        // Step 3: Combine the call graphs together.
        // For each wrapping middleware, determine which request-scoped and singleton components
        // must actually be passed down through the `Next<_>` parameter to the downstream
        // stages of the pipeline.
        //
        // In order to pull this off, we walk the chain in reverse order and accumulate the set of
        // request-scoped and singleton components that are expected as input.
        let mut middleware_id2prebuilt_rs_ids: IndexMap<ComponentId, IndexSet<ComponentId>> =
            IndexMap::new();

        let mut state_accumulator = IndexSet::new();
        let mut wrapping_id2next_field_types: HashMap<ComponentId, InputParameters> =
            HashMap::new();
        for middleware_id in grouped_by_stage.iter().rev() {
            let call_graph = &id2call_graphs[middleware_id];

            let mut prebuilt_ids = IndexSet::new();
            let required_scope_ids =
                extract_request_scoped_compute_nodes(&call_graph.call_graph, component_db)
                    .collect_vec();
            for &request_scoped_id in &required_scope_ids {
                let users = &request_scoped_id2users[&request_scoped_id];
                let n_users = users.len();
                if &users[0] != middleware_id {
                    prebuilt_ids.insert(request_scoped_id);
                } else {
                    if n_users > 1
                        && !component_db
                            .hydrated_component(*middleware_id, computation_db)
                            .is_wrapping_middleware()
                    {
                        prebuilt_ids.insert(request_scoped_id);
                    }
                }
            }
            middleware_id2prebuilt_rs_ids.insert(*middleware_id, prebuilt_ids.clone());

            // We recompute the call graph for the middleware,
            // this time with the right set of prebuilt
            // request-scoped components.
            // This is necessary because the required long-lived inputs may change based on what's already prebuilt!
            let call_graph = request_scoped_call_graph(
                *middleware_id,
                &prebuilt_ids,
                &error_observer_ids,
                computation_db,
                component_db,
                constructible_db,
                diagnostics,
            )?;
            id2call_graphs.insert(*middleware_id, call_graph);

            if component_db
                .hydrated_component(*middleware_id, computation_db)
                .is_wrapping_middleware()
            {
                wrapping_id2next_field_types.insert(
                    *middleware_id,
                    InputParameters::from_iter(state_accumulator.iter()),
                );

                // We are done with these components, no one needs them upstream.
                for request_scoped_id in required_scope_ids {
                    let users = &request_scoped_id2users[&request_scoped_id];
                    if &users[0] == middleware_id {
                        state_accumulator.shift_remove(
                            component_db
                                .hydrated_component(request_scoped_id, computation_db)
                                .output_type()
                                .unwrap(),
                        );
                    }
                }
            }
            extract_long_lived_inputs(
                &id2call_graphs[middleware_id].call_graph,
                component_db,
                &mut state_accumulator,
            );
        }

        // Since we now know which request-scoped components are prebuilt for each middleware, we can
        // compute the final call graph for each of them.
        // In particular, we can determine the concrete type of the generic parameter of the
        // `Next<_>` parameter (that we will codegen later on).
        let mut wrapping_id2next_state = HashMap::new();
        let mut wrapping_id2bound_id = HashMap::new();
        let mut wrapping_id = 0;
        let mut id2ordered_call_graphs = IndexMap::with_capacity(grouped_by_stage.len());
        for middleware_id in grouped_by_stage.iter() {
            let new_middleware_id = if let HydratedComponent::WrappingMiddleware(_) =
                component_db.hydrated_component(*middleware_id, computation_db)
            {
                let next_state_parameters = &wrapping_id2next_field_types[middleware_id];
                let next_state_type = PathType {
                    package_id: PackageId::new(GENERATED_APP_PACKAGE_ID),
                    rustdoc_id: None,
                    base_type: vec![
                        "crate".into(),
                        module_name.clone(),
                        format!("Next{wrapping_id}"),
                    ],
                    generic_arguments: next_state_parameters
                        .lifetimes
                        .iter()
                        .map(|s| {
                            GenericArgument::Lifetime(GenericLifetimeParameter::Named(s.to_owned()))
                        })
                        .collect(),
                };

                // We register a constructor, in order to make it possible to build an instance of
                // `next_type`.
                let next_state_constructor = Callable {
                    is_async: false,
                    takes_self_as_ref: false,
                    path: next_state_type.resolved_path(),
                    output: Some(next_state_type.clone().into()),
                    inputs: next_state_parameters
                        .iter()
                        .map(|input| input.type_.clone())
                        .collect(),
                    invocation_style: InvocationStyle::StructLiteral {
                        field_names: next_state_parameters
                            .iter()
                            .map(|input| (input.ident.clone(), input.type_.clone()))
                            .collect::<BTreeMap<_, _>>(),
                        // TODO: remove when TAIT stabilises
                        extra_field2default_value: {
                            BTreeMap::from([("next".into(), stage_names[wrapping_id + 1].clone())])
                        },
                    },
                    source_coordinates: None,
                };
                let next_state_callable_id = computation_db.get_or_intern(next_state_constructor);
                let next_state_scope_id = component_db.scope_id(*middleware_id);
                let next_state_constructor_id = component_db
                    .get_or_intern_constructor(
                        next_state_callable_id,
                        Lifecycle::RequestScoped,
                        next_state_scope_id,
                        CloningStrategy::NeverClone,
                        computation_db,
                        framework_item_db,
                        None,
                    )
                    .unwrap();
                constructible_db.insert(next_state_constructor_id, component_db, computation_db);

                // Since we now have the concrete type of the generic in `Next<_>`, we can bind
                // the generic type parameter of the middleware to that concrete type.
                let HydratedComponent::WrappingMiddleware(mw) =
                    component_db.hydrated_component(*middleware_id, computation_db)
                else {
                    unreachable!()
                };
                let next_input = &mw.input_types()[mw.next_input_index()];
                let next_generic_parameters = next_input.unassigned_generic_type_parameters();

                #[cfg(debug_assertions)]
                assert_eq!(
                    next_generic_parameters.len(),
                    1,
                    "Next<_> should have exactly one unassigned generic type parameter"
                );

                let next_generic_parameter =
                    next_generic_parameters.iter().next().unwrap().to_owned();

                let mut bindings = HashMap::with_capacity(1);
                bindings.insert(next_generic_parameter, next_state_type.clone().into());
                let bound_middleware_id = component_db.bind_generic_type_parameters(
                    *middleware_id,
                    &bindings,
                    computation_db,
                    framework_item_db,
                );

                let HydratedComponent::WrappingMiddleware(bound_mw) =
                    component_db.hydrated_component(bound_middleware_id, computation_db)
                else {
                    unreachable!()
                };
                // Force the constructibles database to bind a constructor for `Next<{NextState}>`.
                // Really ugly, but alas.
                assert!(constructible_db
                    .get_or_try_bind(
                        next_state_scope_id,
                        &bound_mw.next_input_type().to_owned(),
                        component_db,
                        computation_db,
                        framework_item_db,
                    )
                    .is_some());

                wrapping_id += 1;
                wrapping_id2next_state.insert(
                    bound_middleware_id,
                    NextState {
                        type_: next_state_type,
                        field_bindings: next_state_parameters.bindings.clone(),
                    },
                );
                wrapping_id2bound_id.insert(*middleware_id, bound_middleware_id);
                bound_middleware_id
            } else {
                // Nothing to do for other middlewares/handlers.
                *middleware_id
            };

            let prebuilt_request_scoped_ids = &middleware_id2prebuilt_rs_ids[middleware_id];

            let middleware_call_graph = request_scoped_ordered_call_graph(
                new_middleware_id,
                prebuilt_request_scoped_ids,
                &error_observer_ids,
                computation_db,
                component_db,
                constructible_db,
                package_graph,
                krate_collection,
                diagnostics,
            )?;

            id2ordered_call_graphs.insert(new_middleware_id, middleware_call_graph);
        }

        let stages = {
            let mut stages = vec![];
            let mut post_processing_ids = vec![];
            for middleware_id in &ordered_by_registration {
                let middleware_id = wrapping_id2bound_id
                    .get(middleware_id)
                    .unwrap_or(middleware_id);
                match component_db.hydrated_component(*middleware_id, computation_db) {
                    HydratedComponent::RequestHandler(_)
                    | HydratedComponent::WrappingMiddleware(_) => {
                        let stage_id = stages.len();
                        let input_parameters = if stage_id == 0 {
                            assert!(post_processing_ids.is_empty());
                            InputParameters::from_iter(
                                id2ordered_call_graphs[middleware_id]
                                    .call_graph
                                    .required_input_types(),
                            )
                        } else {
                            let previous_stage: &Stage = &stages[stage_id - 1];
                            let bindings =
                                &previous_stage.next_state.as_ref().unwrap().field_bindings;
                            InputParameters::from_iter(
                                bindings.0.iter().map(|binding| &binding.type_),
                            )
                        };
                        stages.push(Stage {
                            name: stage_names[stage_id].clone(),
                            wrapping_id: *middleware_id,
                            input_parameters,
                            next_state: wrapping_id2next_state.remove(middleware_id),
                            post_processing_ids: std::mem::take(&mut post_processing_ids),
                        });
                    }
                    HydratedComponent::PostProcessingMiddleware(_) => {
                        post_processing_ids.push(*middleware_id);
                    }
                    _ => unreachable!(),
                }
            }
            assert!(post_processing_ids.is_empty());
            stages
        };

        let self_ = Self {
            module_name,
            id2call_graph: id2ordered_call_graphs,
            stages,
        };

        self_.enforce_invariants(component_db, computation_db);

        Ok(self_)
    }

    fn enforce_invariants(&self, component_db: &ComponentDb, computation_db: &ComputationDb) {
        let request_scoped_ids2n_invocations = self
            .id2call_graph
            .values()
            .flat_map(|g| g.call_graph.node_weights())
            .filter_map(|node| {
                let CallGraphNode::Compute { component_id, .. } = node else {
                    return None;
                };
                if component_db.lifecycle(*component_id) != Lifecycle::RequestScoped {
                    return None;
                }
                let component = component_db.hydrated_component(*component_id, computation_db);
                let HydratedComponent::Constructor(constructor) = &component else {
                    return None;
                };
                let Computation::Callable(_) = &constructor.0 else {
                    return None;
                };
                Some(component_id)
            })
            .fold(HashMap::new(), |mut acc, id| {
                *acc.entry(id).or_insert(0) += 1;
                acc
            });

        for (id, n_invocations) in request_scoped_ids2n_invocations {
            if n_invocations > 1 {
                let component = component_db.hydrated_component(*id, computation_db);
                let HydratedComponent::Constructor(constructor) = &component else {
                    unreachable!()
                };
                let Computation::Callable(callable) = &constructor.0 else {
                    unreachable!()
                };
                let path = callable.path.to_string();
                let message = format!(
                    "Request-scoped component `{}` should be invoked at most once in a request pipeline, but it's invoked {} times instead.",
                    path, n_invocations
                );
                panic!("{}", message);
            }
        }
    }
}

impl RequestHandlerPipeline {
    /// Iterate over all the call graphs in the pipeline.
    ///
    /// The order is consistent across invocations, but it should not be assumed to be
    /// invocation order.
    pub(crate) fn graph_iter(&self) -> impl Iterator<Item = &OrderedCallGraph> + '_ {
        self.id2call_graph.values()
    }

    /// Print a representation of the pipeline in graphviz's .DOT format, geared towards
    /// debugging.
    #[allow(unused)]
    pub(crate) fn print_debug_dot(
        &self,
        component_db: &ComponentDb,
        computation_db: &ComputationDb,
    ) {
        for graph in self.graph_iter() {
            graph.print_debug_dot(component_db, computation_db)
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct InputParameters {
    pub(crate) bindings: Bindings,
    pub(crate) lifetimes: IndexSet<String>,
}

impl InputParameters {
    /// Given a set of input types, determine the minimal set of types that should be used as input
    /// parameters for a function or a method.
    ///
    /// In particular:
    ///
    /// - if both `T` and `&T` are needed, only `T` should appear as a field type.
    /// - if both `T` and `&mut T` are needed, only `T` should appear as a field type marked as mutable.
    pub(crate) fn from_iter<'a, T>(types: impl IntoIterator<Item = T>) -> Self
    where
        T: AsRef<ResolvedType>,
    {
        let input_parameters: Vec<_> = Self::get_input_types(types).into_iter().collect();
        let mut lifetimes = IndexSet::new();
        let mut lifetime_generator = LifetimeGenerator::new();
        let input_parameters = input_parameters
            .into_iter()
            .map(|input| {
                let Binding {
                    ident,
                    mut type_,
                    mutable,
                } = input;
                let lifetime2binding: IndexMap<_, _> = type_
                    .named_lifetime_parameters()
                    .into_iter()
                    .map(|lifetime| (lifetime, lifetime_generator.next()))
                    .collect();
                type_.rename_lifetime_parameters(&lifetime2binding);
                lifetimes.extend(lifetime2binding.values().cloned());

                if type_.has_implicit_lifetime_parameters() {
                    let implicit_lifetime_binding = lifetime_generator.next();
                    lifetimes.insert(implicit_lifetime_binding.clone());
                    type_.set_implicit_lifetimes(implicit_lifetime_binding);
                }
                Binding {
                    ident,
                    type_,
                    mutable,
                }
            })
            .collect();

        Self {
            bindings: Bindings(input_parameters),
            lifetimes,
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &Binding> {
        self.bindings.0.iter()
    }

    fn get_input_types<T>(types: impl IntoIterator<Item = T>) -> IndexSet<Binding>
    where
        T: AsRef<ResolvedType>,
    {
        struct Metadata {
            by_value: bool,
            mutable: bool,
        }

        let mut inner_type2by_value: IndexMap<ResolvedType, Metadata> = IndexMap::new();
        for ty_ in types {
            let ty_ = ty_.as_ref();
            if let ResolvedType::Reference(ref_) = ty_ {
                if !ref_.lifetime.is_static() {
                    let entry = inner_type2by_value
                        .entry(ref_.inner.as_ref().to_owned())
                        .or_insert(Metadata {
                            by_value: false,
                            mutable: false,
                        });
                    entry.mutable |= ref_.is_mutable;
                    continue;
                }
            }
            let entry = inner_type2by_value
                .entry(ty_.to_owned())
                .or_insert(Metadata {
                    by_value: true,
                    mutable: false,
                });
            entry.by_value = true;
        }
        inner_type2by_value
            .into_iter()
            .enumerate()
            .map(|(i, (ty_, metadata))| {
                let (ty_, mutable_binding) = if metadata.by_value {
                    (ty_, metadata.mutable)
                } else {
                    (
                        ResolvedType::Reference(TypeReference {
                            is_mutable: metadata.mutable,
                            lifetime: Lifetime::Elided,
                            inner: Box::new(ty_),
                        }),
                        false,
                    )
                };
                Binding {
                    ident: format!("s_{i}"),
                    type_: ty_,
                    mutable: mutable_binding,
                }
            })
            .collect()
    }
}

/// A binding attaches a name to an instance of a type.  
/// We use it to represent the input parameters of a function or a method,
/// fields of a struct, or a variable binding (e.g. `let mut foo: Foo = ...`).
///
/// It can optionally be marked as mutable to allow for mutation of the instance.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) struct Binding {
    /// The binding name.
    ///
    /// E.g. `foo` in `foo: Foo`.
    pub(crate) ident: String,
    /// The type bound to the name.
    pub(crate) type_: ResolvedType,
    /// Whether the binding should be marked as mutable with `mut`.
    ///
    /// E.g. `mut foo: Foo` vs `foo: Foo`.
    pub(crate) mutable: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct Bindings(pub(crate) Vec<Binding>);

impl Bindings {
    /// Produce an expression that has the given type.  
    ///
    /// This can either be achieved by referencing an existing binding, or by constructing a new
    /// one either by immutable reference or by mutable reference.
    ///
    /// E.g. if `self` contains `name: Foo`, you get:
    ///
    /// - `name` if the caller wants an instance of `Foo`
    /// - `&name` if the caller wants an instance of `&Foo`
    /// - `&mut name` if the caller wants an instance of `&mut Foo`
    ///
    /// In the last case, the binding is automatically marked as mutable.
    pub(crate) fn get_expr_for_type(&mut self, type_: &ResolvedType) -> Option<syn::Expr> {
        let binding = self.0.iter().find(|binding| binding.type_ == *type_);
        if let Some(binding) = binding {
            let ident: syn::Expr = syn::parse_str(&binding.ident).unwrap();
            let block = quote! { #ident };
            return Some(syn::parse2(block).unwrap());
        }

        // No exact match.
        // But if they want a reference, perhaps we can borrow something.
        let ResolvedType::Reference(ref_) = type_ else {
            return None;
        };
        let binding = self
            .0
            .iter_mut()
            .find(|binding| ref_.inner.as_ref() == &binding.type_)?;
        let mut_ = if ref_.is_mutable {
            binding.mutable = true;
            Some(quote! { mut })
        } else {
            None
        };
        let ident: syn::Expr = syn::parse_str(&binding.ident).unwrap();
        let block = quote! { &#mut_ #ident };
        Some(syn::parse2(block).unwrap())
    }

    /// Return the first binding with a given type.
    pub(crate) fn find_exact_by_type(&self, type_: &ResolvedType) -> Option<&Binding> {
        self.0.iter().find(|binding| binding.type_ == *type_)
    }
}

/// Extract the set of singleton and request-scoped components that are used as inputs
/// in the provided call graph.
///
/// The extracted component ids are inserted into the provided buffer set.
fn extract_long_lived_inputs(
    call_graph: &RawCallGraph,
    component_db: &ComponentDb,
    buffer: &mut IndexSet<ResolvedType>,
) {
    for node in call_graph.node_weights() {
        let CallGraphNode::InputParameter { type_, source } = node else {
            continue;
        };
        let InputParameterSource::Component(component_id) = source else {
            continue;
        };
        assert_ne!(
            component_db.lifecycle(*component_id),
            Lifecycle::Transient,
            "Transient components should not appear as inputs in a call graph"
        );
        buffer.insert(type_.to_owned());
    }
}

fn extract_request_scoped_compute_nodes<'a>(
    call_graph: &'a RawCallGraph,
    component_db: &'a ComponentDb,
) -> impl Iterator<Item = ComponentId> + 'a {
    call_graph.node_weights().filter_map(move |node| {
        let CallGraphNode::Compute { component_id, .. } = node else {
            return None;
        };
        let Component::Constructor { .. } = component_db[*component_id] else {
            return None;
        };
        if component_db.lifecycle(*component_id) == Lifecycle::RequestScoped {
            Some(*component_id)
        } else {
            None
        }
    })
}
