use crate::compiler::analyses::call_graph::ApplicationStateCallGraph;
use crate::compiler::analyses::components::HydratedComponent;
use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::processing_pipeline::RequestHandlerPipeline;
use crate::compiler::computation::Computation;
use crate::compiler::utils::get_ok_variant;
use crate::diagnostic::{CompilerDiagnostic, DiagnosticSink};
use indexmap::IndexSet;
use miette::Severity;
use pavex_bp_schema::{Lint, LintSetting};

/// Emit a warning for each user-registered constructor that hasn't
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
    // Some user-registered constructors are never used directly—e.g. a constructor with
    // unassigned generic parameters.
    // We consider the original user-registered constructor as "used" if one of its derivations
    // is used.
    let mut used_user_constructor_ids = IndexSet::new();

    let graphs = pipelines
        .flat_map(|p| p.graph_iter())
        .chain(std::iter::once(&application_state_call_graph.call_graph));

    for graph in graphs {
        for node in graph.call_graph.node_weights() {
            let Some(component_id) = node.component_id() else {
                continue;
            };
            let HydratedComponent::Constructor(_) =
                component_db.hydrated_component(component_id, computation_db)
            else {
                continue;
            };
            used_user_constructor_ids.insert(
                component_db
                    .derived_from(&component_id)
                    .unwrap_or(component_id),
            );
        }
    }

    for (id, _) in component_db.constructors(computation_db) {
        if component_db.derived_from(&id).is_some() || component_db.is_framework_primitive(&id) {
            // It's not a user-registered constructor.
            continue;
        }
        if used_user_constructor_ids.contains(&id) {
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
    let source = diagnostics.annotated(
        db.registration_target(user_id),
        "The unused constructor was registered here",
    );
    let HydratedComponent::Constructor(constructor) = db.hydrated_component(id, computation_db)
    else {
        return;
    };
    let output_type = constructor.output_type();
    let output_type = if output_type.is_result() {
        get_ok_variant(output_type)
    } else {
        output_type
    };
    let Computation::Callable(callable) = &constructor.0 else {
        return;
    };
    let output_type = output_type.display_for_error();
    let error = anyhow::anyhow!(
        "You registered a constructor for `{output_type}`, \
    but it's never used.\n\
    `{}` is never invoked since no component is asking for `{output_type}` to be injected as one of its inputs.",
        &callable.path,
    );
    let help = if db.registration(user_id).kind.is_blueprint() {
        "If you want to ignore this warning, call `.ignore(Lint::Unused)` on the registered constructor."
    } else {
        "If you want to ignore this warning, add `allow(unused)` to your constructor attribute."
    }
    .to_string();
    let builder = CompilerDiagnostic::builder(error)
        .optional_source(source)
        .severity(Severity::Warning)
        .help(help);
    diagnostics.push(builder.build())
}
