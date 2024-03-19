use std::collections::BTreeMap;

use ahash::{HashMap, HashMapExt};
use guppy::graph::PackageGraph;
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};

use pavex_bp_schema::{CloningStrategy, Lifecycle};

use crate::compiler::analyses::call_graph::{
    request_scoped_call_graph, request_scoped_ordered_call_graph, CallGraphNode,
    InputParameterSource, OrderedCallGraph, RawCallGraph,
};
use crate::compiler::analyses::components::HydratedComponent;
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::compiler::analyses::framework_items::FrameworkItemDb;
use crate::compiler::app::GENERATED_APP_PACKAGE_ID;
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

pub struct Stage {
    /// The name of the function that represents this stage.
    pub(crate) name: String,
    /// Either a wrapping middleware or a request handler.
    pub(crate) wrapping_id: ComponentId,
    /// Only set if `component_id` is a wrapping middleware.
    pub(crate) next_state: Option<NextState>,
    /// Post-processing middlewares to be invoked after `computation_id` has completed,
    /// but before returning to the caller.
    pub(crate) post_processing_ids: Vec<ComponentId>,
}

impl Stage {
    pub(crate) fn input_parameters(
        &self,
        id2call_graphs: &IndexMap<ComponentId, OrderedCallGraph>,
    ) -> InputParameters {
        let iter = std::iter::once(&id2call_graphs[&self.wrapping_id])
            .chain(
                self.post_processing_ids
                    .iter()
                    .map(|id| &id2call_graphs[id]),
            )
            // TODO: Remove `Response` from the list of input types.
            .map(|g| g.required_input_types().into_iter())
            .flatten();
        InputParameters::from_iter(iter)
    }
}

/// The "state" for `Next<T>` is the concrete type for `T` used in a specific middleware invocation.
///
/// It is computed on a per-pipeline and per-middleware basis, in order to pass down the
/// strict minimum of request-scoped and singleton components that are needed by the downstream
/// stages of the pipeline.
pub(crate) struct NextState {
    pub(crate) type_: PathType,
    /// The state is always a struct.
    /// This map contains the bindings for the fields of the struct: the field name and the type
    /// of the field.
    pub(crate) field_bindings: BTreeMap<String, ResolvedType>,
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
        let ordered_by_registration = component_db
            .middleware_chain(handler_id)
            .unwrap()
            .to_owned();

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

        // Wrapping middlewares -> request handler -> Post-processing middlewares
        let ordered_by_invocation: Vec<_> = ordered_by_registration
            .iter()
            .filter(|id| {
                component_db
                    .hydrated_component(**id, computation_db)
                    .is_wrapping_middleware()
            })
            .chain(std::iter::once(&handler_id))
            .chain(ordered_by_registration.iter().filter(|id| {
                component_db
                    .hydrated_component(**id, computation_db)
                    .is_post_processing_middleware()
            }))
            .collect();

        // Step 2: For each middleware, build an ordered call graph.
        // We need to make sure that request-scoped components are built at most once per request,
        // no matter *where* they are initialised in the overall pipeline.
        // In order to do that, we:
        // - track which request-scoped components are initialised in each middleware in
        //   `request_scoped_ids`
        // - pass `request_scoped_ids` to the downstream stages in the pipeline, to make sure they
        //   don't initialise the same components again but rather reuse the instances created
        //   by the upstream stages.
        let mut request_scoped_prebuilt_ids = IndexSet::new();
        let mut id2call_graphs = HashMap::with_capacity(ordered_by_invocation.len());

        for &component_id in ordered_by_invocation.iter() {
            let call_graph = request_scoped_call_graph(
                *component_id,
                &request_scoped_prebuilt_ids,
                &error_observer_ids,
                computation_db,
                component_db,
                constructible_db,
                diagnostics,
            )?;

            // Add all request-scoped components initialised by this component to the set.
            extract_request_scoped_compute_nodes(
                &call_graph.call_graph,
                component_db,
                &mut request_scoped_prebuilt_ids,
            );

            id2call_graphs.insert(*component_id, call_graph);
        }

        // Step 3: Combine the call graphs together.
        // For each wrapping middleware, determine which request-scoped and singleton components
        // must actually be passed down through the `Next<_>` parameter to the downstream
        // stages of the pipeline.
        //
        // In order to pull this off, we walk the chain in reverse order and accumulate the set of
        // request-scoped and singleton components that are expected as input.
        let mut long_lived_types: IndexSet<ResolvedType> = IndexSet::new();
        let mut wrapping_id2next_field_types: IndexMap<ComponentId, InputParameters> =
            IndexMap::new();
        for &middleware_id in ordered_by_invocation.iter().rev() {
            let call_graph = &id2call_graphs[middleware_id];

            if let HydratedComponent::WrappingMiddleware(_) =
                component_db.hydrated_component(*middleware_id, computation_db)
            {
                wrapping_id2next_field_types.insert(
                    *middleware_id,
                    InputParameters::from_iter(long_lived_types.iter()),
                );
            }

            // Remove all the request-scoped components initialised by this middleware from the set.
            // They can't be needed upstream since they were initialised here!
            for node in call_graph.call_graph.node_weights() {
                if let CallGraphNode::Compute { component_id, .. } = node {
                    if component_db.lifecycle(*component_id) == Lifecycle::RequestScoped {
                        let component =
                            component_db.hydrated_component(*component_id, computation_db);
                        if let Some(output_type) = component.output_type() {
                            long_lived_types.shift_remove(output_type);
                            // We also need to remove the corresponding &-ref type from the set.
                            let ref_output_type = ResolvedType::Reference(TypeReference {
                                is_mutable: false,
                                lifetime: Lifetime::Elided,
                                inner: Box::new(output_type.to_owned()),
                            });
                            long_lived_types.shift_remove(&ref_output_type);
                        }
                    }
                }
            }

            // But this middleware can in turn ask for some long-lived components to be passed
            // down from the upstream stages of the pipeline, therefore we need to add those to
            // the set.
            extract_long_lived_inputs(&call_graph.call_graph, component_db, &mut long_lived_types);
        }
        wrapping_id2next_field_types.reverse();

        // Determine, for each middleware, if they consume a request-scoped component
        // that is also needed by later stages of the pipeline.

        // Since we now know which request-scoped components are needed by each middleware, we can
        // make the call graph for each middleware concrete—i.e. we can replace the generic
        // `Next<_>` parameter with a concrete type (that we will codegen later on).
        let mut wrapping_id2next_state = HashMap::new();
        let mut wrapping_id2bound_id = HashMap::new();
        let mut request_scoped_prebuilt_ids = IndexSet::new();
        let mut wrapping_id = 0;
        let mut id2ordered_call_graphs = IndexMap::with_capacity(ordered_by_invocation.len());
        for &middleware_id in ordered_by_invocation.iter() {
            let middleware_id = if let HydratedComponent::WrappingMiddleware(_) =
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
                        .map(|s| GenericArgument::Lifetime(s.to_owned()))
                        .collect(),
                };

                // We register a constructor, in order to make it possible to build an instance of
                // `next_type`.
                let next_state_bindings = next_state_parameters
                    .iter()
                    .map(|input| (input.ident.clone(), input.type_.clone()))
                    .collect::<BTreeMap<_, _>>();
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
                        field_names: next_state_bindings.clone(),
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
                        field_bindings: next_state_bindings,
                    },
                );
                wrapping_id2bound_id.insert(*middleware_id, bound_middleware_id);
                bound_middleware_id
            } else {
                // Nothing to do for other middlewares/handlers.
                *middleware_id
            };

            let middleware_call_graph = request_scoped_ordered_call_graph(
                middleware_id,
                &request_scoped_prebuilt_ids,
                &error_observer_ids,
                computation_db,
                component_db,
                constructible_db,
                package_graph,
                krate_collection,
                diagnostics,
            )?;
            // Add all request-scoped components initialised by this middleware to the set.
            extract_request_scoped_compute_nodes(
                &middleware_call_graph.call_graph,
                component_db,
                &mut request_scoped_prebuilt_ids,
            );
            id2ordered_call_graphs.insert(middleware_id, middleware_call_graph);
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
                        stages.push(Stage {
                            name: stage_names[stage_id].clone(),
                            wrapping_id: *middleware_id,
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

        Ok(Self {
            module_name,
            id2call_graph: id2ordered_call_graphs,
            stages,
        })
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

pub(crate) struct InputParameters {
    pub(crate) input_parameters: IndexSet<InputParameter>,
    pub(crate) lifetimes: IndexSet<GenericLifetimeParameter>,
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
        let input_parameters = Self::get_input_types(types);
        let mut lifetimes = IndexSet::new();
        let mut lifetime_generator = LifetimeGenerator::new();
        let input_parameters = input_parameters
            .into_iter()
            .map(|input| {
                let InputParameter {
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
                InputParameter {
                    ident,
                    type_,
                    mutable,
                }
            })
            .collect();

        Self {
            input_parameters,
            lifetimes: lifetimes
                .into_iter()
                .map(|s| GenericLifetimeParameter::Named(s))
                .collect(),
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &InputParameter> {
        self.input_parameters.iter()
    }

    fn get_input_types<T>(types: impl IntoIterator<Item = T>) -> IndexSet<InputParameter>
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
                InputParameter {
                    ident: format!("s_{i}"),
                    type_: ty_,
                    mutable: mutable_binding,
                }
            })
            .collect()
    }
}

/// An input parameter for a function or a method.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) struct InputParameter {
    /// The binding name.
    ///
    /// E.g. `foo` in `foo: Foo`.
    pub(crate) ident: String,
    /// The type that should be taken as input.
    pub(crate) type_: ResolvedType,
    /// Whether the binding should be marked as mutable with `mut`.
    ///
    /// E.g. `mut foo: Foo` vs `foo: Foo`.
    pub(crate) mutable: bool,
}

/// Extract the set of request-scoped and singleton components that are used as inputs
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

fn extract_request_scoped_compute_nodes(
    call_graph: &RawCallGraph,
    component_db: &ComponentDb,
    buffer: &mut IndexSet<ComponentId>,
) {
    for node in call_graph.node_weights() {
        let CallGraphNode::Compute { component_id, .. } = node else {
            continue;
        };
        if component_db.lifecycle(*component_id) == Lifecycle::RequestScoped {
            buffer.insert(*component_id);
        }
    }
}
