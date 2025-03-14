use std::collections::{BTreeMap, BTreeSet};

use ahash::{HashMap, HashMapExt, HashSet};
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use quote::quote;

use pavex_bp_schema::{CloningStrategy, Lifecycle};
use tracing::Level;

use crate::compiler::analyses::call_graph::{
    CallGraphNode, InputParameterSource, OrderedCallGraph, RawCallGraph, RawCallGraphExt,
    request_scoped_call_graph, request_scoped_ordered_call_graph,
};
use crate::compiler::analyses::components::HydratedComponent;
use crate::compiler::analyses::components::component::Component;
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::compiler::analyses::framework_items::FrameworkItemDb;
use crate::compiler::app::GENERATED_APP_PACKAGE_ID;
use crate::compiler::computation::Computation;
use crate::compiler::utils::LifetimeGenerator;
use crate::diagnostic::{
    self, AnnotatedSource, CompilerDiagnostic, HelpWithSnippet, OptionalLabeledSpanExt,
    OptionalSourceSpanExt,
};
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
    /// Associate each component id with the call graph for that component.
    /// It is guaranteed to be in invocation order.
    pub(crate) id2call_graph: IndexMap<ComponentId, OrderedCallGraph>,
    /// The different stages of the pipeline, in invocation order.
    pub(crate) stages: Vec<Stage>,
    /// Associate each component id with the name of the generated function that wraps around
    /// it to build its dependencies.
    /// It is guaranteed to be in invocation order.
    pub(crate) id2name: IndexMap<ComponentId, String>,
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
    /// Post-processing middlewares to be invoked after `wrapping_id` has completed,
    /// but before returning to the caller.
    pub(crate) post_processing_ids: Vec<ComponentId>,
    /// Pre-processing middlewares to be invoked before `wrapping_id` has completed.
    pub(crate) pre_processing_ids: Vec<ComponentId>,
    /// Where to insert `.clone()` invocations for inputs ahead of invoking a
    /// middleware within this stage.
    /// Middlewares are indexed based on their position in the invocation order for
    /// this stage.
    pub(crate) type2cloning_indexes: IndexMap<ResolvedType, BTreeSet<usize>>,
}

#[derive(Debug)]
struct PipelineIds(Vec<StageIds>);

impl PipelineIds {
    fn invocation_order(&self) -> Vec<ComponentId> {
        let mut ordered = Vec::new();
        // First add all pre-processing middlewares and wrapping middlewares.
        for stage_ids in &self.0 {
            ordered.extend(stage_ids.pre_processing_ids.iter().cloned());
            ordered.push(stage_ids.middle_id);
        }
        // Then add all post-processing middlewares, in reverse order.
        for stage_ids in self.0.iter().rev() {
            ordered.extend(stage_ids.post_processing_ids.iter().cloned());
        }
        ordered
    }
}

#[derive(Debug)]
struct StageIds {
    pre_processing_ids: Vec<ComponentId>,
    /// Either a wrapping middleware or a request handler.
    middle_id: ComponentId,
    post_processing_ids: Vec<ComponentId>,
}

impl StageIds {
    fn invocation_order(&self) -> Vec<ComponentId> {
        self.pre_processing_ids
            .iter()
            .cloned()
            .chain(std::iter::once(self.middle_id))
            .chain(self.post_processing_ids.iter().cloned())
            .collect()
    }
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
        krate_collection: &CrateCollection,
        diagnostics: &mut crate::diagnostic::DiagnosticSink,
    ) -> Result<Self, ()> {
        let error_observer_ids = component_db.error_observers(handler_id).unwrap().to_owned();

        // Step 1a: Determine the sequence of middlewares that the request handler is wrapped in.
        let ordered_by_registration = {
            let mut v = component_db
                .middleware_chain(handler_id)
                .unwrap()
                .to_owned();
            v.push(handler_id);
            v
        };

        // Step 1b: Assign a unique name to each stage, with a suffix based on their
        // execution order.
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
                    HydratedComponent::PreProcessingMiddleware(_)
                    | HydratedComponent::PostProcessingMiddleware(_) => {}
                    _ => unreachable!(),
                };
            }
            stage_names
        };

        // Step 1c: Group middlewares around in stages. Each stage corresponds to a wrapping
        // middleware (or the request handler) alongside the pre- and post- middlewares
        // that execute immediately before/after it.
        //
        // Invariant: all middlewares in stage N are wrapped by the wrapping
        // middleware from stage N-1.
        let pipeline_ids = {
            let first = component_db
                .hydrated_component(*ordered_by_registration.first().unwrap(), computation_db);
            assert!(
                matches!(first, HydratedComponent::WrappingMiddleware(_)),
                "First component should be a wrapping middleware, but it's a {:?}",
                first
            );
            let mut stage_ids = vec![];
            let mut pres = vec![];
            let mut posts = vec![];
            for id in ordered_by_registration.iter() {
                let component = component_db.hydrated_component(*id, computation_db);
                match component {
                    HydratedComponent::PreProcessingMiddleware(_) => {
                        pres.push(*id);
                    }
                    HydratedComponent::RequestHandler(_)
                    | HydratedComponent::WrappingMiddleware(_) => {
                        stage_ids.push(StageIds {
                            pre_processing_ids: std::mem::take(&mut pres),
                            middle_id: *id,
                            post_processing_ids: std::mem::take(&mut posts),
                        });
                    }
                    HydratedComponent::PostProcessingMiddleware(_) => {
                        posts.push(*id);
                    }
                    _ => unreachable!(),
                }
            }
            assert!(stage_ids[0].pre_processing_ids.is_empty());
            assert!(stage_ids[0].post_processing_ids.is_empty());
            PipelineIds(stage_ids)
        };

        // Step 2: For each middleware, build an ordered call graph.
        // We associate each request-scoped component with the stage
        // where it must be built and the number of users.
        let mut request_scoped_id2state_stage_index: HashMap<ComponentId, (usize, usize)> =
            HashMap::new();
        let mut id2call_graphs = HashMap::new();

        for (stage_index, stage_ids) in pipeline_ids.0.iter().enumerate().rev() {
            for middleware_id in stage_ids.invocation_order().into_iter().rev() {
                let call_graph = request_scoped_call_graph(
                    middleware_id,
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
                    let build_in = if component_db.is_wrapping_middleware(middleware_id) {
                        stage_index
                    } else {
                        stage_index - 1
                    };
                    let entry = request_scoped_id2state_stage_index
                        .entry(request_scoped_id)
                        .or_insert((build_in, 0));
                    entry.0 = entry.0.min(build_in);
                    entry.1 += 1;
                }

                id2call_graphs.insert(middleware_id, call_graph);
            }
        }

        let request_scoped2built_at_stage_index: HashMap<ComponentId, usize> =
            request_scoped_id2state_stage_index
                .into_iter()
                .filter_map(|(request_scoped_id, (stage_index, n_users))| {
                    // If a request-scoped component is used by more than one middleware, it must be
                    // built at (or before) the stage where it is first used.
                    // If a request-scoped component is used by only one middleware, it can be built
                    // directly in the "closure" of that middleware. No need to pass it down the pipeline.
                    if n_users > 1 {
                        Some((request_scoped_id, stage_index))
                    } else {
                        None
                    }
                })
                .collect();

        // Step 3: Combine the call graphs together.
        // For each wrapping middleware, determine which request-scoped and singleton components
        // must actually be passed down through the `Next<_>` parameter to the downstream
        // stages of the pipeline.
        //
        // In order to pull this off, we walk the chain in reverse order and accumulate the set of
        // request-scoped and singleton components that are expected as input.
        let mut previous_next_state = None;
        let mut wrapping_id = stage_names.len() - 2;
        let mut wrapping_id2bound_id = HashMap::new();
        let mut wrapping_id2next_state = HashMap::new();
        let mut id2ordered_call_graphs = IndexMap::new();
        let mut state_accumulator = IndexSet::new();

        for (stage_index, stage_ids) in pipeline_ids.0.iter().enumerate().rev() {
            for mut middleware_id in stage_ids.invocation_order().into_iter().rev() {
                let call_graph = &id2call_graphs[&middleware_id];

                let mut prebuilt_ids: IndexSet<ComponentId> = IndexSet::new();
                let required_scope_ids: HashSet<_> =
                    extract_request_scoped_compute_nodes(&call_graph.call_graph, component_db)
                        .collect();
                for (request_scoped_id, &built_at) in &request_scoped2built_at_stage_index {
                    if built_at < stage_index {
                        prebuilt_ids.insert(*request_scoped_id);
                    } else if built_at == stage_index
                        && required_scope_ids.contains(request_scoped_id)
                    {
                        assert!(component_db.is_wrapping_middleware(middleware_id));
                    }
                }

                if let Some(next_state_parameters) = &previous_next_state {
                    if let Some(bound_middleware_id) = Self::bind_next(
                        module_name.clone(),
                        middleware_id,
                        next_state_parameters,
                        wrapping_id,
                        &stage_names,
                        &mut wrapping_id2next_state,
                        computation_db,
                        component_db,
                        constructible_db,
                        framework_item_db,
                    ) {
                        if stage_index != 0 {
                            wrapping_id -= 1;
                        }
                        wrapping_id2bound_id.insert(middleware_id, bound_middleware_id);
                        middleware_id = bound_middleware_id;
                    }
                }

                // We recompute the call graph for the middleware,
                // this time with the right set of prebuilt
                // request-scoped components.
                // This is necessary because the required long-lived inputs may change based on what's already prebuilt!
                let middleware_call_graph = request_scoped_ordered_call_graph(
                    middleware_id,
                    &prebuilt_ids,
                    &error_observer_ids,
                    computation_db,
                    component_db,
                    constructible_db,
                    krate_collection,
                    diagnostics,
                )?;
                extract_long_lived_inputs(
                    &middleware_call_graph.call_graph,
                    component_db,
                    &mut state_accumulator,
                );
                id2ordered_call_graphs.insert(middleware_id, middleware_call_graph);
            }

            state_accumulator.shift_remove(&component_db.pavex_response);

            if stage_index == 0 {
                continue;
            }
            previous_next_state = {
                let inputs = state_accumulator.iter().filter(|ty| match ty {
                    ResolvedType::ResolvedPath(_) => *ty != &component_db.pavex_response,
                    ResolvedType::Reference(ref_) => {
                        ref_.inner.as_ref() != &component_db.pavex_response
                    }
                    _ => true,
                });
                let inputs = InputParameters::from_iter(inputs);
                if tracing::event_enabled!(Level::DEBUG) {
                    let mut buffer = String::new();
                    inputs.bindings.0.iter().map(|b| &b.type_).for_each(|t| {
                        use std::fmt::Write as _;
                        writeln!(&mut buffer, "- {:?}", t).unwrap();
                    });
                    tracing::debug!(
                        "The `Next` state parameter for {} contains:\n{buffer}",
                        stage_names[stage_index],
                    );
                }
                Some(inputs)
            };

            for (id, built_at) in &request_scoped2built_at_stage_index {
                if *built_at == stage_index - 1 {
                    let output = component_db
                        .hydrated_component(*id, computation_db)
                        .output_type()
                        .cloned()
                        .unwrap();
                    let to_be_removed: Vec<_> = state_accumulator
                        .iter()
                        .filter(|ty| match ty {
                            ResolvedType::ResolvedPath(_) => *ty == &output,
                            ResolvedType::Reference(ref_) => ref_.inner.as_ref() == &output,
                            _ => false,
                        })
                        .cloned()
                        .collect();
                    for ty in to_be_removed {
                        state_accumulator.shift_remove(&ty);
                    }
                }
            }
        }

        let mut stages = {
            let mut stages = vec![];
            let mut post_processing_ids = vec![];
            let mut pre_processing_ids = vec![];
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
                            assert!(pre_processing_ids.is_empty());
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
                            pre_processing_ids: std::mem::take(&mut pre_processing_ids),
                            // This will be populated later on.
                            type2cloning_indexes: Default::default(),
                        });
                    }
                    HydratedComponent::PostProcessingMiddleware(_) => {
                        post_processing_ids.push(*middleware_id);
                    }
                    HydratedComponent::PreProcessingMiddleware(_) => {
                        pre_processing_ids.push(*middleware_id);
                    }
                    _ => unreachable!(),
                }
            }
            assert!(post_processing_ids.is_empty());
            stages
        };

        // Step 4: at this stage, each call graph *in isolation* satisfies
        // the constraints of the borrow checker.
        // That's not enough though: different middlewares from the same stage
        // might be trying to take the same type value using move semantics.
        // We don't know if that's allowed since we haven't done any cross-graph
        // analysis up to this stage.
        // Now it's the moment!

        // We iterate in reverse order because closer to the request handler
        // we are less likely to encounter borrowing issues that relate to some of
        // our synthetic types.
        'stage_iter: for stage in stages.iter_mut().rev() {
            let ids: Vec<_> = stage
                .pre_processing_ids
                .iter()
                .chain(std::iter::once(&stage.wrapping_id))
                .chain(stage.post_processing_ids.iter())
                .collect();

            #[derive(Debug)]
            struct CloningInfo {
                /// The indexes of the middlewares that take the type as input by value.
                consumed_by: Vec<ConsumerInfo>,
                /// The indexes of the middlewares that take a reference (mutable or not)
                /// to the type *after* it has been consumed at least once by another
                /// middleware.
                ref_by: Vec<usize>,
            }

            #[derive(Clone, Copy, Debug)]
            struct ConsumerInfo {
                middleware_index: usize,
                /// The id of the component for this input type within
                /// the context of this middleware.
                /// Yes, it can change across middlewares!
                component_id: ComponentId,
            }

            let mut type2info = HashMap::<ResolvedType, CloningInfo>::new();

            for (index, &id) in ids.iter().enumerate() {
                let call_graph = &id2ordered_call_graphs[id];
                let input_types =
                    call_graph
                        .call_graph
                        .node_weights()
                        .filter_map(|node| match node {
                            CallGraphNode::Compute { .. } | CallGraphNode::MatchBranching => None,
                            CallGraphNode::InputParameter { type_, source } => match source {
                                InputParameterSource::Component(id) => Some((type_.clone(), *id)),
                                InputParameterSource::External => None,
                            },
                        });

                for (ty, component_id) in input_types {
                    match ty {
                        ResolvedType::Reference(ref_) => {
                            // We recurse through multi-references (e.g. &&&T).
                            let mut inner = ref_.inner.as_ref();
                            while let ResolvedType::Reference(ref_) = inner {
                                inner = ref_.inner.as_ref();
                            }
                            if let Some(info) = type2info.get_mut(inner.as_ref()) {
                                info.ref_by.push(index);
                            }
                        }
                        ResolvedType::ResolvedPath(_) |
                        ResolvedType::Tuple(_) => {
                            let info = type2info.entry(ty.clone()).or_insert(
                                CloningInfo { consumed_by: Vec::new(), ref_by: Vec::new() }
                            );
                            info.consumed_by.push(ConsumerInfo { middleware_index: index, component_id });
                        }
                        // Scalars are trivially `Copy`, this analysis doesn't concern them.
                        ResolvedType::ScalarPrimitive(_) => {
                            continue;
                        }
                        // We'd never encounter a raw slice as input type.
                        ResolvedType::Slice(_) |
                        // All types are concrete at this stage.
                        ResolvedType::Generic(_) => unreachable!(),
                    }
                }
            }

            let mut type2cloning_indexes = IndexMap::with_capacity(type2info.len());
            for (ty_, cloning_info) in type2info.into_iter() {
                let mut indexes = cloning_info.consumed_by;

                // The type is never borrowed after the last move,
                // thus we don't need a `.clone()` on the invocation
                // for the last consumer
                match cloning_info.ref_by.last() {
                    Some(&last_ref_index) => {
                        if last_ref_index < indexes.last().unwrap().middleware_index {
                            indexes.pop();
                        }
                    }
                    None => {
                        indexes.pop();
                    }
                }

                if indexes.is_empty() {
                    continue;
                }

                let issue = indexes.iter().find_position(|info| {
                    component_db.cloning_strategy(info.component_id) == CloningStrategy::NeverClone
                });
                if let Some((issue_index, info)) = issue {
                    let next_ref = cloning_info
                        .ref_by
                        .iter()
                        .find(|&ix| *ix > info.middleware_index);
                    let next_move = indexes
                        .get(issue_index + 1)
                        .map(|info| &info.middleware_index);
                    let next_index = match (next_ref, next_move) {
                        (None, None) => unreachable!(),
                        (None, Some(ix)) | (Some(ix), None) => ix,
                        (Some(m), Some(r)) => m.min(r),
                    };
                    let moved_by = *ids[info.middleware_index];
                    let later_used_by = *ids[*next_index];
                    emit_cloning_error(
                        &ty_,
                        moved_by,
                        later_used_by,
                        info.component_id,
                        component_db,
                        computation_db,
                        diagnostics,
                    );
                    // We emit at most one error to minimise the likelihood of
                    // duplicates/error cascades that stem from the same underlying
                    // issue.
                    break 'stage_iter;
                }

                let indexes: BTreeSet<_> =
                    indexes.into_iter().map(|v| v.middleware_index).collect();
                type2cloning_indexes.insert(ty_, indexes);
            }
            stage.type2cloning_indexes = type2cloning_indexes;
        }

        let id2name: IndexMap<_, _> = {
            let mut wrapping_index = 0u32;
            let mut pre_processing_index = 0u32;
            let mut post_processing_index = 0u32;
            let mut get_ident = |id| match component_db.hydrated_component(id, computation_db) {
                HydratedComponent::WrappingMiddleware(_) => {
                    let ident = format!("wrapping_{}", wrapping_index);
                    wrapping_index += 1;
                    ident
                }
                HydratedComponent::PostProcessingMiddleware(_) => {
                    let ident = format!("post_processing_{}", post_processing_index);
                    post_processing_index += 1;
                    ident
                }
                HydratedComponent::PreProcessingMiddleware(_) => {
                    let ident = format!("pre_processing_{}", pre_processing_index);
                    pre_processing_index += 1;
                    ident
                }
                HydratedComponent::RequestHandler(_) => "handler".to_string(),
                _ => unreachable!(),
            };

            pipeline_ids
                .invocation_order()
                .into_iter()
                .map(|id| {
                    let id = *wrapping_id2bound_id.get(&id).unwrap_or(&id);
                    (id, get_ident(id))
                })
                .collect()
        };

        // We re-order id2ordered_call_graph to be in invocation order, re-using the fact that
        // id2names is already in invocation order.
        let id2ordered_call_graphs = id2name
            .iter()
            .map(|(id, _)| (*id, id2ordered_call_graphs.shift_remove(id).unwrap()))
            .collect();

        let self_ = Self {
            module_name,
            id2call_graph: id2ordered_call_graphs,
            stages,
            id2name,
        };

        self_.enforce_invariants(component_db, computation_db);

        Ok(self_)
    }

    /// Bind the generic parameter of `Next` to the concrete type of the next state.
    /// Return `None` if the middleware is not a wrapping middleware.
    fn bind_next(
        module_name: String,
        middleware_id: ComponentId,
        next_state_parameters: &InputParameters,
        wrapping_id: usize,
        stage_names: &[String],
        wrapping_id2next_state: &mut HashMap<ComponentId, NextState>,
        computation_db: &mut ComputationDb,
        component_db: &mut ComponentDb,
        constructible_db: &mut ConstructibleDb,
        framework_item_db: &FrameworkItemDb,
    ) -> Option<ComponentId> {
        if !matches!(
            component_db.hydrated_component(middleware_id, computation_db),
            HydratedComponent::WrappingMiddleware(_)
        ) {
            return None;
        }

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
                .map(|s| GenericArgument::Lifetime(GenericLifetimeParameter::Named(s.to_owned())))
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
        let next_state_scope_id = component_db.scope_id(middleware_id);
        let next_state_constructor_id = component_db
            .get_or_intern_constructor_without_validation(
                next_state_callable_id,
                Lifecycle::RequestScoped,
                next_state_scope_id,
                CloningStrategy::NeverClone,
                computation_db,
                None,
            )
            .unwrap();
        constructible_db.insert(next_state_constructor_id, component_db, computation_db);

        // Since we now have the concrete type of the generic in `Next<_>`, we can bind
        // the generic type parameter of the middleware to that concrete type.
        let HydratedComponent::WrappingMiddleware(mw) =
            component_db.hydrated_component(middleware_id, computation_db)
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

        let next_generic_parameter = next_generic_parameters.iter().next().unwrap().to_owned();

        let mut bindings = HashMap::with_capacity(1);
        bindings.insert(next_generic_parameter, next_state_type.clone().into());
        let bound_middleware_id = component_db.bind_generic_type_parameters(
            middleware_id,
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
        assert!(
            constructible_db
                .get_or_try_bind(
                    next_state_scope_id,
                    &bound_mw.next_input_type().to_owned(),
                    component_db,
                    computation_db,
                    framework_item_db,
                )
                .is_some()
        );

        wrapping_id2next_state.insert(
            bound_middleware_id,
            NextState {
                type_: next_state_type,
                field_bindings: next_state_parameters.bindings.clone(),
            },
        );

        Some(bound_middleware_id)
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
                for call_graph in self.id2call_graph.values() {
                    call_graph.print_debug_dot(&path, component_db, computation_db);
                }
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
        for (i, graph) in self.graph_iter().enumerate() {
            graph.print_debug_dot(&i.to_string(), component_db, computation_db)
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
    pub(crate) fn from_iter<T>(types: impl IntoIterator<Item = T>) -> Self
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

        if let Some(binding) = self
            .0
            .iter_mut()
            .find(|binding| ref_.inner.as_ref() == &binding.type_)
        {
            let mut_ = if ref_.is_mutable {
                binding.mutable = true;
                Some(quote! { mut })
            } else {
                None
            };
            let ident: syn::Expr = syn::parse_str(&binding.ident).unwrap();
            let block = quote! { &#mut_ #ident };
            return Some(syn::parse2(block).unwrap());
        }

        // If we are loooking for a `&T` and we have a `&mut T`,
        // we can use the latter as the former thanks to Rust's coercion rules.
        if !ref_.is_mutable {
            let new_ref = ResolvedType::Reference(TypeReference {
                is_mutable: true,
                lifetime: ref_.lifetime.clone(),
                inner: Box::new(ref_.inner.as_ref().clone()),
            });
            let binding = self.0.iter().find(|binding| binding.type_ == new_ref);
            if let Some(binding) = binding {
                let ident: syn::Expr = syn::parse_str(&binding.ident).unwrap();
                let block = quote! { #ident };
                return Some(syn::parse2(block).unwrap());
            }
        }

        None
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
        if let InputParameterSource::Component(component_id) = source {
            assert_ne!(
                component_db.lifecycle(*component_id),
                Lifecycle::Transient,
                "Transient components should not appear as inputs in a call graph"
            );
        };
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

fn emit_cloning_error(
    ty_: &ResolvedType,
    moved_by: ComponentId,
    later_used_by: ComponentId,
    component_id: ComponentId,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    let Computation::Callable(moved_by_callable) = component_db
        .hydrated_component(moved_by, computation_db)
        .computation()
    else {
        unreachable!()
    };
    let moved_by_path = &moved_by_callable.path;

    let Computation::Callable(later_used_by_callable) = component_db
        .hydrated_component(later_used_by, computation_db)
        .computation()
    else {
        unreachable!()
    };
    let later_used_by_path = &later_used_by_callable.path;

    // TODO(diagnostics): improve the error message pointing at the specific components that require
    //  the contested type.
    let error_msg = format!(
        "I can't generate code that will pass the borrow checker *and* match the instructions \
        in your blueprint:\n\
        - One of the components in the call graph for `{moved_by_path}` consumes `{ty_:?}` by value\n\
        - But, later on, the same type is used in the call graph of `{later_used_by_path}`.\n\
        You forbid cloning of `{ty_:?}`, therefore I can't resolve this conflict."
    );

    let mut diagnostic = CompilerDiagnostic::builder(anyhow::anyhow!(error_msg));
    if let Some(user_component_id) = component_db.user_component_id(component_id) {
        let help_msg = format!(
            "Allow me to clone `{ty_:?}` in order to satisfy the borrow checker.\n\
                You can do so by invoking `.clone_if_necessary()` after having registered your constructor.",
        );
        let location = component_db
            .user_component_db()
            .get_location(user_component_id);
        let source = diagnostics.source(location).map(|s| {
            diagnostic::f_macro_span(s.source(), location)
                .labeled("The constructor was registered here".into())
                .attach(s)
        });
        let help = match source {
            None => HelpWithSnippet::new(help_msg, AnnotatedSource::empty()),
            Some(source) => HelpWithSnippet::new(help_msg, source).normalize(),
        };
        diagnostic = diagnostic.help_with_snippet(help);
    }

    diagnostics.push(diagnostic.build());
}
