use miette::NamedSource;
use pavex::blueprint::constructor::Lifecycle;

use crate::{
    compiler::{
        analyses::{
            components::{ComponentDb, ComponentId},
            computations::ComputationDb,
        },
        computation::Computation,
    },
    diagnostic::{AnnotatedSnippet, CompilerDiagnosticBuilder, HelpWithSnippet},
};

pub(super) fn suggest_wrapping_in_a_smart_pointer(
    consumed_component_id: Option<ComponentId>,
    component_db: &ComponentDb,
    computation_db: &ComputationDb,
    mut diagnostic: CompilerDiagnosticBuilder,
) -> CompilerDiagnosticBuilder {
    if let Some(consumed_component_id) = consumed_component_id {
        let lifecycle = component_db.lifecycle(consumed_component_id);
        let component = component_db.hydrated_component(consumed_component_id, computation_db);
        let type_ = component.output_type();
        let is_framework = matches!(component.computation(), Computation::FrameworkItem(_));
        // All singletons are cloneable, by construction.
        // And the user can't control whether a framework type does or doesn't implement Clone.
        if lifecycle != Some(&Lifecycle::Singleton) && !is_framework {
            let ref_counting_help = format!("If `{type_:?}` itself cannot implement `Clone`, consider wrapping it in an `std::sync::Rc` or `std::sync::Arc`.");
            let ref_counting_help = HelpWithSnippet::new(
                ref_counting_help,
                AnnotatedSnippet::new_with_labels(NamedSource::new("", ""), vec![]),
            );
            diagnostic = diagnostic.help_with_snippet(ref_counting_help);
        }
    }
    diagnostic
}
