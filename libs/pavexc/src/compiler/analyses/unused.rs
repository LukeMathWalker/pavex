use crate::compiler::analyses::call_graph::ApplicationStateCallGraph;
use crate::compiler::analyses::components::HydratedComponent;
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::processing_pipeline::RequestHandlerPipeline;
use crate::compiler::utils::get_ok_variant;
use crate::diagnostic::{CompilerDiagnostic, DiagnosticSink};
use indexmap::IndexSet;
use miette::Severity;
use pavex_bp_schema::{Lint, LintSetting};

use super::components::component::Component;

/// Emit a warning for each user-registered constructor and prebuilt type that hasn't
/// been used in the code-generated pipelines.
pub(crate) fn detect_unused<'a, I>(
    pipelines: I,
    application_state_call_graph: &ApplicationStateCallGraph,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    diagnostics: &DiagnosticSink,
) where
    I: Iterator<Item = &'a RequestHandlerPipeline>,
{
    // Some user-registered components are never used directly—e.g. a constructor with
    // unassigned generic parameters.
    // We consider the original user-registered component as "used" if one of its derivations
    // is used.
    let mut used_user_components_ids = IndexSet::new();

    let graphs = pipelines
        .flat_map(|p| p.graph_iter())
        .chain(std::iter::once(&application_state_call_graph.call_graph));

    for graph in graphs {
        for node in graph.call_graph.node_weights() {
            let Some(component_id) = node.component_id() else {
                continue;
            };
            match component_db.hydrated_component(component_id, computation_db) {
                HydratedComponent::Constructor(_) | HydratedComponent::PrebuiltType(_) => {}
                _ => {
                    continue;
                }
            }
            used_user_components_ids.insert(
                component_db
                    .derived_from(&component_id)
                    .unwrap_or(component_id),
            );
        }
    }

    for (id, c) in component_db.iter() {
        if !matches!(
            c,
            Component::Constructor { .. } | Component::PrebuiltType { .. }
        ) {
            continue;
        }
        if component_db.derived_from(&id).is_some() || component_db.is_framework_primitive(&id) {
            // It's not a user-registered component..
            continue;
        }
        if used_user_components_ids.contains(&id) {
            continue;
        }

        if let Some(overrides) = component_db.lints(id) {
            if overrides.get(&Lint::Unused) == Some(&LintSetting::Allow) {
                // No warning!
                continue;
            }
        }

        emit_unused_warning(id, component_db, computation_db, diagnostics);
    }
}

fn emit_unused_warning(
    id: ComponentId,
    db: &ComponentDb,
    computation_db: &ComputationDb,
    diagnostics: &crate::diagnostic::DiagnosticSink,
) {
    let Some(user_id) = db.user_component_id(id) else {
        return;
    };
    let component_kind = db.user_db()[user_id].kind();
    let source = diagnostics.annotated(
        db.registration_target(user_id),
        format!("The unused {component_kind} was registered here"),
    );
    let component = db.hydrated_component(id, computation_db);
    let (err_msg, ty_) = match component {
        HydratedComponent::Constructor(constructor) => {
            let output_type = constructor.output_type();
            let ty_ = if output_type.is_result() {
                get_ok_variant(output_type)
            } else {
                output_type
            };
            let err_msg = format!(
                "You registered a constructor for `{}`, but it's never used.",
                ty_.display_for_error()
            );
            (err_msg, ty_.to_owned())
        }
        HydratedComponent::PrebuiltType(ty_) => {
            let err_msg = format!(
                "You marked `{}` as prebuilt, but it's never used.",
                ty_.display_for_error()
            );
            (err_msg, ty_.into_owned())
        }
        _ => unreachable!(),
    };
    let error = anyhow::anyhow!(
        "{err_msg}\n\
        No component is asking for `{}` to be injected as one of its inputs.",
        ty_.display_for_error()
    );

    let help = if db.registration(user_id).kind.is_blueprint() {
        "If you want to ignore this warning, invoke `.allow(Lint::Unused)` on the component."
    } else {
        "If you want to ignore this warning, add `allow(unused)` to the attribute you used to define the component."
    }
    .to_string();
    let builder = CompilerDiagnostic::builder(error)
        .optional_source(source)
        .severity(Severity::Warning)
        .help(help);
    diagnostics.push(builder.build())
}
