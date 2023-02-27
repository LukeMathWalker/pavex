use std::collections::BTreeSet;

use ahash::{HashMap, HashMapExt, HashSet};
use bimap::BiHashMap;
use convert_case::{Case, Casing};
use guppy::PackageId;
use indexmap::{IndexMap, IndexSet};
use petgraph::Direction;
use proc_macro2::Ident;

use pavex_builder::Lifecycle;

use crate::language::{
    Callable, GenericArgument, InvocationStyle, ResolvedPath, ResolvedPathSegment,
    ResolvedPathType, ResolvedType,
};
use crate::rustdoc::CORE_PACKAGE_ID;
use crate::web::analyses::call_graph::{
    build_call_graph, CallGraph, CallGraphNode, NumberOfAllowedInvocations,
};
use crate::web::analyses::components::{ComponentDb, ComponentId, HydratedComponent};
use crate::web::analyses::computations::ComputationDb;
use crate::web::analyses::constructibles::ConstructibleDb;
use crate::web::app::GENERATED_APP_PACKAGE_ID;
use crate::web::computation::Computation;

/// Build a [`CallGraph`] for the application state.
#[tracing::instrument(name = "compute_application_state_call_graph", skip_all)]
pub(crate) fn application_state_call_graph(
    runtime_singleton_bindings: &BiHashMap<Ident, ResolvedType>,
    computation_db: &mut ComputationDb,
    component_db: &mut ComponentDb,
    constructible_db: &mut ConstructibleDb,
) -> ApplicationStateCallGraph {
    fn lifecycle2invocations(lifecycle: &Lifecycle) -> Option<NumberOfAllowedInvocations> {
        match lifecycle {
            Lifecycle::Singleton => Some(NumberOfAllowedInvocations::One),
            Lifecycle::RequestScoped => {
                panic!("Singletons should not depend on types with a request-scoped lifecycle.")
            }
            Lifecycle::Transient => {
                panic!("Singletons should not depend on types with a transient lifecycle.")
            }
        }
    }

    // We build a "mock" callable that has the right inputs in order to drive the machinery
    // that builds the dependency graph.
    let package_id = PackageId::new(GENERATED_APP_PACKAGE_ID);
    let application_state_type = ResolvedPathType {
        package_id: package_id.clone(),
        rustdoc_id: None,
        base_type: vec!["crate".into(), "ApplicationState".into()],
        generic_arguments: vec![],
    };
    let application_state_constructor = Callable {
        is_async: false,
        path: application_state_type.resolved_path(),
        output: Some(application_state_type.clone().into()),
        inputs: runtime_singleton_bindings.right_values().cloned().collect(),
        invocation_style: InvocationStyle::StructLiteral {
            field_names: runtime_singleton_bindings
                .iter()
                .map(|(ident, type_)| (ident.to_string(), type_.to_owned()))
                .collect(),
        },
    };
    let application_state_callable_id = computation_db.get_or_intern(application_state_constructor);
    let application_state_id = component_db
        .get_or_intern_constructor(
            application_state_callable_id,
            Lifecycle::Singleton,
            computation_db,
        )
        .unwrap();
    let CallGraph {
        call_graph,
        root_node_index,
    } = build_call_graph(
        application_state_id,
        computation_db,
        component_db,
        constructible_db,
        lifecycle2invocations,
    );

    // We need to make sure that all paths return the same output type.
    // For `ApplicationState`, that's either `ApplicationState` or `Result<ApplicationState, E>`,
    // where `E` is an error enum with a variant for each possible error type that might be
    // encountered when building `ApplicationState`.

    // Let's start by collecting the possible error types.
    let error_type2err_match_ids = {
        let mut map = IndexMap::<_, HashSet<ComponentId>>::new();
        let mut output_node_indexes = call_graph
            .externals(Direction::Outgoing)
            .collect::<BTreeSet<_>>();
        // We only care about errors at this point.
        output_node_indexes.remove(&root_node_index);
        for output_node_index in output_node_indexes {
            let CallGraphNode::Compute {
                component_id,
                ..
            } = &call_graph[output_node_index] else {
                unreachable!()
            };
            let component = component_db.hydrated_component(*component_id, computation_db);
            assert!(
                matches!(
                    component,
                    HydratedComponent::Transformer(Computation::MatchResult(_)),
                ),
                "One of the output components is not a `MatchResult` transformer: {:?}",
                component
            );
            map.entry(component.output_type().to_owned())
                .or_default()
                .insert(*component_id);
        }
        map
    };
    let error_types: IndexSet<ResolvedType> = error_type2err_match_ids
        .iter()
        .map(|(e, _)| e.to_owned())
        .collect();

    if error_types.is_empty() {
        // Happy days! Nothing to do!
        return ApplicationStateCallGraph {
            call_graph: CallGraph {
                call_graph,
                root_node_index,
            },
            error_variants: Default::default(),
        };
    }

    let error_enum = ResolvedPathType {
        package_id: package_id.clone(),
        rustdoc_id: None,
        base_type: vec!["crate".into(), "ApplicationStateError".into()],
        generic_arguments: vec![],
    };
    let application_state_result = ResolvedPathType {
        package_id: PackageId::new(CORE_PACKAGE_ID),
        rustdoc_id: None,
        base_type: vec!["core".into(), "result".into(), "Result".into()],
        generic_arguments: vec![
            GenericArgument::AssignedTypeParameter(application_state_type.clone().into()),
            GenericArgument::AssignedTypeParameter(error_enum.clone().into()),
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
            output: Some(application_state_result.clone().into()),
            path: ResolvedPath {
                segments: ok_wrapper_path,
                qualified_self: None,
                package_id: PackageId::new(CORE_PACKAGE_ID),
            },
            inputs: vec![application_state_type.into()],
            invocation_style: InvocationStyle::FunctionCall,
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
            output: Some(application_state_result.into()),
            path: ResolvedPath {
                segments: err_wrapper_path,
                qualified_self: None,
                package_id: PackageId::new(CORE_PACKAGE_ID),
            },
            inputs: vec![error_enum.clone().into()],
            invocation_style: InvocationStyle::FunctionCall,
        }
    };
    component_db.get_or_intern_transformer(
        computation_db.get_or_intern(ok_wrapper),
        application_state_id,
        computation_db,
    );

    let mut error_variants = IndexMap::new();
    for (error_type, err_match_ids) in &error_type2err_match_ids {
        let mut collision_map = HashMap::<_, usize>::new();
        for err_match_id in err_match_ids {
            let fallible_id = component_db.fallible_id(*err_match_id);
            let fallible = component_db.hydrated_component(fallible_id, computation_db);
            let fallible_callable = match &fallible {
                HydratedComponent::Constructor(c) => {
                    let Computation::Callable(c) = &c.0 else { unreachable!() };
                    c
                }
                HydratedComponent::RequestHandler(r) => &r.callable,
                HydratedComponent::ErrorHandler(_) | HydratedComponent::Transformer(_) => {
                    unreachable!()
                }
            };
            let error_type_name = fallible_callable
                .path
                .segments
                .last()
                .unwrap()
                .ident
                .to_case(Case::Pascal);
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
                    package_id: package_id.clone(),
                },
                output: Some(error_enum.clone().into()),
                inputs: vec![error_type.to_owned()],
                invocation_style: InvocationStyle::FunctionCall,
            };
            let transformer_id = component_db.get_or_intern_transformer(
                computation_db.get_or_intern(error_variant_constructor.clone()),
                *err_match_id,
                computation_db,
            );
            // We need to do an Err(..) wrap around the error variant returned by the transformer.
            component_db.get_or_intern_transformer(
                computation_db.get_or_intern(err_wrapper.clone()),
                transformer_id,
                computation_db,
            );
        }
    }

    // With all the transformers in place, we can now build the final call graph!
    let call_graph = build_call_graph(
        application_state_id,
        computation_db,
        component_db,
        constructible_db,
        lifecycle2invocations,
    );

    ApplicationStateCallGraph {
        call_graph,
        error_variants,
    }
}

pub(crate) struct ApplicationStateCallGraph {
    pub(crate) call_graph: CallGraph,
    pub(crate) error_variants: IndexMap<String, ResolvedType>,
}
