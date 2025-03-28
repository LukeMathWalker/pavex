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
    let registration = db.registration(user_id);
    let source = diagnostics.annotated(
        TargetSpan::Registration(registration),
        format!("The {kind} was registered here"),
    );
    // Match the casing that you would use in each circumstance.
    let (clone_if_necessary, never_clone) = if registration.kind.from_attribute() {
        ("clone_if_necessary", "never_clone")
    } else {
        ("CloneIfNecessary", "NeverClone")
    };
    let output_type = type_.display_for_error();
    let error_msg = match kind {
        ComponentKind::Constructor => {
            let callable_path = &computation_db[user_id].path;
            format!(
                "`{output_type}` doesn't implement the `Clone` trait, but its constructor, `{callable_path}`, is marked as `{clone_if_necessary}`."
            )
        }
        ComponentKind::PrebuiltType => {
            format!(
                "`{output_type}` doesn't implement the `Clone` trait, but it's marked as `{clone_if_necessary}`."
            )
        }
        ComponentKind::ConfigType => {
            format!(
                "All configuration types must be clonable.\n\
                `{output_type}` is a configuration type, but it doesn't implement the `Clone` trait.",
            )
        }
        _ => unreachable!(),
    };
    let e = anyhow::anyhow!(e).context(error_msg);
    let help = (kind != ComponentKind::ConfigType).then(|| {
        format!("Implement the `Clone` trait for `{output_type}`, or mark it as `{never_clone}`.",)
    });
    let diagnostic = CompilerDiagnostic::builder(e)
        .optional_source(source)
        .optional_help(help)
        .build();
    diagnostics.push(diagnostic);
}
