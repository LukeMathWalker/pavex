use guppy::graph::PackageGraph;
use indexmap::IndexSet;

use crate::{
    compiler::{
        analyses::{
            components::{ComponentDb, ComponentId, HydratedComponent},
            computations::ComputationDb,
        },
        computation::Computation,
        traits::{assert_trait_is_implemented, MissingTraitImplementationError},
        utils::process_framework_path,
    },
    diagnostic::{self, CallableType, CompilerDiagnostic, OptionalSourceSpanExt},
    language::ResolvedType,
    rustdoc::CrateCollection,
    try_source,
};

/// Verify that all singletons needed at runtime implement `Send` and `Sync`.
/// This is required since Pavex runs on a multi-threaded `tokio` runtime.
#[tracing::instrument(name = "Verify `Send` and `Sync` for runtime singletons", skip_all)]
pub(crate) fn runtime_singletons_are_thread_safe(
    runtime_singletons: &IndexSet<(ResolvedType, ComponentId)>,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
    diagnostics: &mut Vec<miette::Error>,
) {
    let send = process_framework_path("core::marker::Send", package_graph, krate_collection);
    let sync = process_framework_path("core::marker::Sync", package_graph, krate_collection);
    for (singleton_type, component_id) in runtime_singletons {
        for trait_ in [&send, &sync] {
            let ResolvedType::ResolvedPath(trait_) = trait_ else {
                unreachable!()
            };
            if let Err(e) = assert_trait_is_implemented(krate_collection, singleton_type, trait_) {
                missing_trait_implementation(
                    e,
                    *component_id,
                    package_graph,
                    component_db,
                    computation_db,
                    diagnostics,
                );
            }
        }
    }
}

fn missing_trait_implementation(
    e: MissingTraitImplementationError,
    component_id: ComponentId,
    package_graph: &PackageGraph,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    diagnostics: &mut Vec<miette::Error>,
) {
    let HydratedComponent::Constructor(c) =
        component_db.hydrated_component(component_id, computation_db)
    else {
        unreachable!()
    };
    let component_id = match c.0 {
        Computation::Callable(_) => component_id,
        Computation::MatchResult(_) => component_db.fallible_id(component_id),
        Computation::PrebuiltType(_) => component_db.derived_from(&component_id).unwrap(),
    };
    let user_component_id = component_db.user_component_id(component_id).unwrap();
    let user_component_db = &component_db.user_component_db();
    let user_component = &user_component_db[user_component_id];
    let component_kind = user_component.callable_type();
    let location = user_component_db.get_location(user_component_id);
    let source = try_source!(location, package_graph, diagnostics);
    let label = source
        .as_ref()
        .map(|source| {
            diagnostic::get_f_macro_invocation_span(&source, location)
                .labeled(format!("The {component_kind} was registered here"))
        })
        .flatten();
    let help = if component_kind == CallableType::PrebuiltType {
        "All prebuilt types that are needed at runtime must implement the `Send` and `Sync` traits.\n\
        Pavex runs on a multi-threaded HTTP server and the application state is shared \
        across all worker threads."
            .into()
    } else {
        "All singletons must implement the `Send` and `Sync` traits.\n\
        Pavex runs on a multi-threaded HTTP server and the application state is shared \
        across all worker threads."
            .into()
    };
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .optional_label(label)
        .help(help)
        .build();
    diagnostics.push(diagnostic.into());
}
