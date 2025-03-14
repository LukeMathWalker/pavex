use indexmap::IndexSet;

use crate::{
    compiler::{
        analyses::{
            components::{ComponentDb, ComponentId, HydratedComponent},
            computations::ComputationDb,
        },
        computation::Computation,
        traits::{MissingTraitImplementationError, assert_trait_is_implemented},
        utils::process_framework_path,
    },
    diagnostic::{
        self, CompilerDiagnostic, ComponentKind, OptionalLabeledSpanExt, OptionalSourceSpanExt,
    },
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
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    let send = process_framework_path("core::marker::Send", krate_collection);
    let sync = process_framework_path("core::marker::Sync", krate_collection);
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
    component_id: ComponentId,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
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
    let component_kind = user_component.kind();
    let location = user_component_db.get_location(user_component_id);
    let source = diagnostics.source(&location).map(|s| {
        diagnostic::f_macro_span(s.source(), location)
            .labeled(format!("The {component_kind} was registered here"))
            .attach(s)
    });
    let help = if component_kind == ComponentKind::PrebuiltType {
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
