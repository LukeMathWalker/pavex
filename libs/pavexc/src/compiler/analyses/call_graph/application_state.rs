use std::collections::{BTreeMap, BTreeSet};

use ahash::{HashMap, HashMapExt};
use convert_case::{Case, Casing};
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};
use petgraph::Direction;

use pavex_bp_schema::{CloningStrategy, Lifecycle};

use crate::compiler::analyses::application_state::ApplicationState;
use crate::compiler::analyses::call_graph::{
    CallGraph, CallGraphNode, NumberOfAllowedInvocations, OrderedCallGraph,
    core_graph::build_call_graph,
};
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::components::{
    ConsumptionMode, HydratedComponent, InsertTransformer,
};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::constructibles::ConstructibleDb;
use crate::compiler::analyses::framework_items::FrameworkItemDb;
use crate::compiler::app::GENERATED_APP_PACKAGE_ID;
use crate::compiler::computation::Computation;
use crate::language::{
    Callable, GenericArgument, InvocationStyle, PathType, ResolvedPath, ResolvedPathSegment,
    ResolvedType,
};
use crate::rustdoc::{CORE_PACKAGE_ID_REPR, CrateCollection};

/// Build an [`OrderedCallGraph`] for the application state.
#[tracing::instrument(name = "Compute the application state graph", skip_all)]
pub(crate) fn application_state_call_graph(
    application_state: &ApplicationState,
    computation_db: &mut ComputationDb,
    component_db: &mut ComponentDb,
    constructible_db: &mut ConstructibleDb,
    framework_item_db: &FrameworkItemDb,
    krate_collection: &CrateCollection,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) -> Result<ApplicationStateCallGraph, ()> {
    fn lifecycle2invocations(lifecycle: Lifecycle) -> Option<NumberOfAllowedInvocations> {
        match lifecycle {
            Lifecycle::Singleton => Some(NumberOfAllowedInvocations::One),
            Lifecycle::Transient => Some(NumberOfAllowedInvocations::Multiple),
            Lifecycle::RequestScoped => {
                // Singletons cannot depend on components with a request-scoped lifecycle.
                // This is enforced before trying to build the call graph, by `ConstructibleDb`, so
                // we don't need to worry about it here.
                unreachable!()
            }
        }
    }

    // We build a "mock" callable that has the right inputs in order to drive the machinery
    // that builds the dependency graph.
    let application_state_id = component_db
        .get_or_intern_constructor(
            computation_db.get_or_intern(application_state.initializer()),
            Lifecycle::Singleton,
            component_db.scope_graph().application_state_scope_id(),
            CloningStrategy::NeverClone,
            computation_db,
            framework_item_db,
            None,
        )
        .unwrap();
    let Ok(CallGraph {
        call_graph,
        root_node_index,
        root_scope_id,
    }) = build_call_graph(
        application_state_id,
        &IndexSet::new(),
        &[],
        computation_db,
        component_db,
        constructible_db,
        lifecycle2invocations,
        diagnostics,
    )
    else {
        return Err(());
    };

    // We need to make sure that all paths return the same output type.
    // For `ApplicationState`, that's either `ApplicationState` or `Result<ApplicationState, E>`,
    // where `E` is an error enum with a variant for each possible error type that might be
    // encountered when building `ApplicationState`.

    // Let's start by collecting the possible error types.
    let error_type2err_match_ids = {
        let mut map = IndexMap::<_, BTreeSet<ComponentId>>::new();
        let mut output_node_indexes = call_graph
            .externals(Direction::Outgoing)
            .collect::<BTreeSet<_>>();
        // We only care about errors at this point.
        output_node_indexes.remove(&root_node_index);
        for output_node_index in output_node_indexes {
            let CallGraphNode::Compute { component_id, .. } = &call_graph[output_node_index] else {
                unreachable!()
            };
            let component = component_db.hydrated_component(*component_id, computation_db);
            assert!(
                matches!(
                    component,
                    HydratedComponent::Transformer(Computation::MatchResult(_), ..),
                ),
                "One of the output components is not a `MatchResult` transformer: {:?}",
                component
            );
            map.entry(component.output_type().unwrap().to_owned())
                .or_default()
                .insert(*component_id);
        }
        map
    };
    let error_types: IndexSet<ResolvedType> = error_type2err_match_ids
        .iter()
        .map(|(e, _)| e.to_owned())
        .collect();

    let (call_graph, error_variants) = if error_types.is_empty() {
        // Happy days! Nothing to do!
        (
            CallGraph {
                call_graph,
                root_node_index,
                root_scope_id,
            },
            Default::default(),
        )
    } else {
        let error_enum = PathType {
            package_id: PackageId::new(GENERATED_APP_PACKAGE_ID),
            rustdoc_id: None,
            base_type: vec!["crate".into(), "ApplicationStateError".into()],
            generic_arguments: vec![],
        };
        let application_state_result = PathType {
            package_id: PackageId::new(CORE_PACKAGE_ID_REPR),
            rustdoc_id: None,
            base_type: vec!["core".into(), "result".into(), "Result".into()],
            generic_arguments: vec![
                GenericArgument::TypeParameter(application_state.type_().into()),
                GenericArgument::TypeParameter(error_enum.clone().into()),
            ],
        };
        // We need to add an `Ok` wrap around `ApplicationState`, since we are returning a `Result`.
        let ok_wrapper = {
            let ok_wrapper_path = {
                let mut v = application_state_result.resolved_path().segments;
                v.push(ResolvedPathSegment {
                    ident: "Ok".into(),
                    generic_arguments: vec![],
                });
                v
            };
            Callable {
                is_async: false,
                takes_self_as_ref: false,
                output: Some(application_state_result.clone().into()),
                path: ResolvedPath {
                    segments: ok_wrapper_path,
                    qualified_self: None,
                    package_id: PackageId::new(CORE_PACKAGE_ID_REPR),
                },
                inputs: vec![application_state.type_().into()],
                invocation_style: InvocationStyle::FunctionCall,
                source_coordinates: None,
            }
        };
        let err_wrapper = {
            let err_wrapper_path = {
                let mut v = application_state_result.resolved_path().segments;
                v.push(ResolvedPathSegment {
                    ident: "Err".into(),
                    generic_arguments: vec![],
                });
                v
            };
            Callable {
                is_async: false,
                takes_self_as_ref: false,
                output: Some(application_state_result.into()),
                path: ResolvedPath {
                    segments: err_wrapper_path,
                    qualified_self: None,
                    package_id: PackageId::new(CORE_PACKAGE_ID_REPR),
                },
                inputs: vec![error_enum.clone().into()],
                invocation_style: InvocationStyle::FunctionCall,
                source_coordinates: None,
            }
        };
        component_db.get_or_intern_transformer(
            computation_db.get_or_intern(ok_wrapper),
            application_state_id,
            component_db.scope_graph().application_state_scope_id(),
            InsertTransformer::Eagerly,
            ConsumptionMode::Move,
            0,
            computation_db,
        );

        let mut error_variants = BTreeMap::new();
        let mut collision_map = HashMap::<_, usize>::new();
        for (error_type, err_match_ids) in &error_type2err_match_ids {
            for err_match_id in err_match_ids {
                let fallible_id = component_db.fallible_id(*err_match_id);
                let fallible = component_db.hydrated_component(fallible_id, computation_db);
                let fallible_callable = match &fallible {
                    HydratedComponent::Constructor(c) => {
                        let Computation::Callable(c) = &c.0 else {
                            unreachable!()
                        };
                        c
                    }
                    HydratedComponent::WrappingMiddleware(w) => &w.callable,
                    HydratedComponent::RequestHandler(r) => &r.callable,
                    HydratedComponent::PostProcessingMiddleware(pp) => &pp.callable,
                    HydratedComponent::PreProcessingMiddleware(pp) => &pp.callable,
                    HydratedComponent::ErrorObserver(_)
                    | HydratedComponent::Transformer(..)
                    | HydratedComponent::ConfigType(..)
                    | HydratedComponent::PrebuiltType(..) => {
                        unreachable!()
                    }
                };
                let error_type_name = {
                    let n_path_segments = fallible_callable.path.segments.len();
                    let last_segment = &fallible_callable.path.segments[n_path_segments - 1]
                        .ident
                        .to_case(Case::Pascal);
                    if n_path_segments >= 3 {
                        let second_to_last_segment =
                            &fallible_callable.path.segments[n_path_segments - 2].ident;
                        if second_to_last_segment.is_case(Case::Pascal) {
                            // This is likely to be a method on a struct/enum.
                            format!("{second_to_last_segment}{last_segment}")
                        } else {
                            last_segment.to_owned()
                        }
                    } else {
                        last_segment.to_owned()
                    }
                };
                let n_duplicates = collision_map.entry(error_type_name.clone()).or_insert(1);
                let error_type_name = if *n_duplicates == 1 {
                    error_type_name
                } else {
                    format!("{error_type_name}{n_duplicates}")
                };
                error_variants.insert(error_type_name.clone(), error_type.clone());
                *n_duplicates += 1;
                let error_variant_constructor = Callable {
                    is_async: false,
                    takes_self_as_ref: false,
                    path: ResolvedPath {
                        segments: vec![
                            ResolvedPathSegment {
                                ident: "crate".into(),
                                generic_arguments: vec![],
                            },
                            ResolvedPathSegment {
                                ident: "ApplicationStateError".into(),
                                generic_arguments: vec![],
                            },
                            ResolvedPathSegment {
                                ident: error_type_name.to_owned(),
                                generic_arguments: vec![],
                            },
                        ],
                        qualified_self: None,
                        package_id: PackageId::new(GENERATED_APP_PACKAGE_ID),
                    },
                    output: Some(error_enum.clone().into()),
                    inputs: vec![error_type.to_owned()],
                    invocation_style: InvocationStyle::FunctionCall,
                    source_coordinates: None,
                };
                let transformer_id = component_db.get_or_intern_transformer(
                    computation_db.get_or_intern(error_variant_constructor.clone()),
                    *err_match_id,
                    component_db.scope_graph().application_state_scope_id(),
                    InsertTransformer::Eagerly,
                    ConsumptionMode::Move,
                    0,
                    computation_db,
                );
                // We need to do an Err(..) wrap around the error variant returned by the transformer.
                component_db.get_or_intern_transformer(
                    computation_db.get_or_intern(err_wrapper.clone()),
                    transformer_id,
                    component_db.scope_graph().application_state_scope_id(),
                    InsertTransformer::Eagerly,
                    ConsumptionMode::Move,
                    0,
                    computation_db,
                );
            }
        }

        // With all the transformers in place, we can now build the final call graph!
        let Ok(cg) = build_call_graph(
            application_state_id,
            &IndexSet::new(),
            &[],
            computation_db,
            component_db,
            constructible_db,
            lifecycle2invocations,
            diagnostics,
        ) else {
            return Err(());
        };

        (cg, error_variants)
    };

    let call_graph = OrderedCallGraph::new(
        call_graph,
        component_db,
        computation_db,
        krate_collection,
        diagnostics,
    )?;
    Ok(ApplicationStateCallGraph {
        call_graph,
        error_variants: error_variants.into_iter().collect(),
    })
}

pub(crate) struct ApplicationStateCallGraph {
    pub(crate) call_graph: OrderedCallGraph,
    pub(crate) error_variants: IndexMap<String, ResolvedType>,
}
