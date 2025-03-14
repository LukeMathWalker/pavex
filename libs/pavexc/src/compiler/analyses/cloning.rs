use pavex_bp_schema::CloningStrategy;

use crate::{
    compiler::{
        analyses::{
            components::{ComponentDb, ComponentId, HydratedComponent},
            computations::ComputationDb,
        },
        traits::{MissingTraitImplementationError, assert_trait_is_implemented},
        utils::process_framework_path,
    },
    diagnostic::{CompilerDiagnostic, ComponentKind, TargetSpan},
    language::ResolvedType,
    rustdoc::CrateCollection,
};

/// Verify that all types whose cloning strategy is set to "CloneIfNecessary" can actually
/// be cloned.
#[tracing::instrument(name = "If cloning is allowed, types must be clonable", skip_all)]
pub(crate) fn clonables_can_be_cloned<'a>(
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    krate_collection: &CrateCollection,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    let clone = process_framework_path("core::clone::Clone", krate_collection);
    let ResolvedType::ResolvedPath(clone) = clone else {
        unreachable!()
    };

    for (id, _) in component_db.iter() {
        let hydrated = component_db.hydrated_component(id, computation_db);
        match hydrated {
            HydratedComponent::Constructor(_) | HydratedComponent::PrebuiltType(_) => {
                if component_db.cloning_strategy(id) != CloningStrategy::CloneIfNecessary {
                    continue;
                }
            }
            HydratedComponent::ConfigType(_) => {}
            _ => {
                continue;
            }
        };
        let Some(output_type) = hydrated.output_type() else {
            continue;
        };
        if let Err(e) = assert_trait_is_implemented(krate_collection, output_type, &clone) {
            must_be_clonable(
                e,
                output_type,
                id,
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
    id: ComponentId,
    db: &ComponentDb,
    computation_db: &ComputationDb,
    diagnostics: &mut crate::diagnostic::DiagnosticSink,
) {
    let id = db.derived_from(&id).unwrap_or(id);
    let user_id = db.user_component_id(id).unwrap();
    let kind = db.user_db()[user_id].kind();
    let source = diagnostics.annotated(
        TargetSpan::Registration(db.registration(user_id)),
        format!("The {kind} was registered here"),
    );
    let error_msg = match kind {
        ComponentKind::Constructor => {
            let callable_path = &computation_db[user_id].path;
            format!(
                "A type must be clonable if you set its cloning strategy to `CloneIfNecessary`.\n\
                The cloning strategy for `{callable_path}` is `CloneIfNecessary`, but `{}`, its output type, doesn't implement the `Clone` trait.",
                type_.display_for_error(),
            )
        }
        ComponentKind::PrebuiltType => {
            format!(
                "A type must be clonable if you set its cloning strategy to `CloneIfNecessary`.\n\
                The cloning strategy for `{}`, a prebuilt type, is `CloneIfNecessary`, but it doesn't implement the `Clone` trait.",
                type_.display_for_error(),
            )
        }
        ComponentKind::ConfigType => {
            format!(
                "All configuration types must be clonable.\n\
                `{}` is a configuration type, but it doesn't implement the `Clone` trait.",
                type_.display_for_error(),
            )
        }
        _ => unreachable!(),
    };
    let e = anyhow::anyhow!(e).context(error_msg);
    let help = (kind != ComponentKind::ConfigType).then(|| {
        format!(
            "Either set the cloning strategy to `NeverClone` or implement `Clone` for `{}`",
            type_.display_for_error()
        )
    });
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .optional_help(help)
        .build();
    diagnostics.push(diagnostic);
}
