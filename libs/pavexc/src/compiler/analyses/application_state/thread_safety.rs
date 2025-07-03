use indexmap::IndexSet;

use crate::{
    compiler::{
        analyses::{
            components::{ComponentDb, ComponentId, HydratedComponent},
            computations::ComputationDb,
        },
        computation::Computation,
        traits::{MissingTraitImplementationError, assert_trait_is_implemented},
        utils::resolve_type_path,
    },
    diagnostic::{CompilerDiagnostic, ComponentKind},
    language::ResolvedType,
    rustdoc::CrateCollection,
};

/// Verify that all singletons needed at runtime implement `Send` and `Sync`.
/// This is required since Pavex runs on a multi-threaded `tokio` runtime.
#[tracing::instrument(name = "Verify `Send` and `Sync` for runtime singletons", skip_all)]
pub(crate) fn runtime_singletons_are_thread_safe(
    runtime_singletons: &IndexSet<(ResolvedType, ComponentId)>,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    krate_collection: &CrateCollection,
    diagnostics: &crate::diagnostic::DiagnosticSink,
) {
    let send = resolve_type_path("core::marker::Send", krate_collection);
    let sync = resolve_type_path("core::marker::Sync", krate_collection);
    for (singleton_type, component_id) in runtime_singletons {
        for trait_ in [&send, &sync] {
            let ResolvedType::ResolvedPath(trait_) = trait_ else {
                unreachable!()
            };
            if let Err(e) = assert_trait_is_implemented(krate_collection, singleton_type, trait_) {
                missing_trait_implementation(
                    e,
                    *component_id,
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
    id: ComponentId,
    db: &ComponentDb,
    computation_db: &ComputationDb,
    diagnostics: &crate::diagnostic::DiagnosticSink,
) {
    let HydratedComponent::Constructor(c) = db.hydrated_component(id, computation_db) else {
        unreachable!()
    };
    let component_id = match c.0 {
        Computation::Callable(_) => id,
        Computation::MatchResult(_) => db.fallible_id(id),
        Computation::PrebuiltType(_) => db.derived_from(&id).unwrap(),
    };
    let user_id = db.user_component_id(component_id).unwrap();
    let kind = db.user_db()[user_id].kind();
    let source = diagnostics.annotated(
        db.registration_target(user_id),
        format!("The {kind} was registered here"),
    );
    let help = if kind == ComponentKind::PrebuiltType {
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
        .help(help)
        .build();
    diagnostics.push(diagnostic);
}
