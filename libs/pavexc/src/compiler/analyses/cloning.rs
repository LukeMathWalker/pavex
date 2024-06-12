use guppy::graph::PackageGraph;
use pavex_bp_schema::CloningStrategy;

use crate::{
    compiler::{
        analyses::{
            components::{ComponentDb, ComponentId, HydratedComponent},
            computations::ComputationDb,
        },
        traits::{assert_trait_is_implemented, MissingTraitImplementationError},
        utils::process_framework_path,
    },
    diagnostic::{self, CallableType, CompilerDiagnostic, OptionalSourceSpanExt},
    language::ResolvedType,
    rustdoc::CrateCollection,
    try_source,
};

/// Verify that all types whose cloning strategy is set to "CloneIfNecessary" can actually
/// be cloned.
#[tracing::instrument(name = "If cloning is allowed, types must be clonable", skip_all)]
pub(crate) fn clonables_can_be_cloned<'a>(
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    package_graph: &PackageGraph,
    krate_collection: &CrateCollection,
    diagnostics: &mut Vec<miette::Error>,
) {
    let clone = process_framework_path("core::clone::Clone", package_graph, krate_collection);
    let ResolvedType::ResolvedPath(clone) = clone else {
        unreachable!()
    };

    for (id, _) in component_db.iter() {
        let HydratedComponent::Constructor(constructor) =
            component_db.hydrated_component(id, computation_db)
        else {
            continue;
        };
        if component_db.cloning_strategy(id) != CloningStrategy::CloneIfNecessary {
            continue;
        }
        let output_type = constructor.output_type();
        if let Err(e) = assert_trait_is_implemented(krate_collection, output_type, &clone) {
            must_be_clonable(
                e,
                output_type,
                id,
                package_graph,
                component_db,
                computation_db,
                diagnostics,
            );
        }
    }
}

fn must_be_clonable(
    e: MissingTraitImplementationError,
    type_: &ResolvedType,
    component_id: ComponentId,
    package_graph: &PackageGraph,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    diagnostics: &mut Vec<miette::Error>,
) {
    let component_id = component_db
        .derived_from(&component_id)
        .unwrap_or(component_id);
    let user_component_id = component_db.user_component_id(component_id).unwrap();
    let user_component_db = &component_db.user_component_db();
    let callable_type = user_component_db[user_component_id].callable_type();
    let location = user_component_db.get_location(user_component_id);
    let source = try_source!(location, package_graph, diagnostics);
    let label = source
        .as_ref()
        .map(|source| {
            diagnostic::get_f_macro_invocation_span(&source, location)
                .labeled(format!("The {callable_type} was registered here"))
        })
        .flatten();
    let error_msg = match callable_type {
        CallableType::Constructor => {
            let callable_path = &computation_db[user_component_id].path;
            format!(
                    "A type must be clonable if you set its cloning strategy to `CloneIfNecessary`.\n\
                    The cloning strategy for `{callable_path}` is `CloneIfNecessary`, but `{}`, its output type, doesn't implement the `Clone` trait.",
                    type_.display_for_error(),
                )
        }
        CallableType::PrebuiltType => {
            format!(
                    "A type must be clonable if you set its cloning strategy to `CloneIfNecessary`.\n\
                    The cloning strategy for `{}`, a prebuilt type, is `CloneIfNecessary`, but it doesn't implement the `Clone` trait.",
                    type_.display_for_error(),
                )
        }
        _ => unreachable!(),
    };
    let e = anyhow::anyhow!(e).context(error_msg);
    let help = format!(
        "Either set the cloning strategy to `NeverClone` or implement `Clone` for `{}`",
        type_.display_for_error()
    );
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .optional_label(label)
        .help(help)
        .build();
    diagnostics.push(diagnostic.into());
}
