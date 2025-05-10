use pavex_bp_schema::{CreatedBy, RawIdentifiers};
use pavexc_attr_parser::AnnotationProperties;

use crate::{
    compiler::analyses::{computations::ComputationDb, user_components::UserComponent},
    rustdoc::CrateCollection,
};

use super::{
    AuxiliaryData, Registration,
    registry::{AnnotatedItem, AnnotationRegistry},
};

/// For each registered component, check if the item it refers to has been annotated with one of
/// Pavex's macros.
///
/// If that's the case, update its configuration accordingly.
/// Properties specified directly on the blueprint have higher priority than ones specified
/// in the annotation.
pub fn augment_from_annotation(
    registry: &AnnotationRegistry,
    aux: &mut AuxiliaryData,
    computation_db: &ComputationDb,
    krate_collection: &CrateCollection,
) {
    let component_ids: Vec<_> = aux.iter().map(|(id, _)| id).collect();
    for id in component_ids {
        if !matches!(&aux[id], UserComponent::WrappingMiddleware { .. }) {
            continue;
        }
        let Some(source_id) = &computation_db[id].source_coordinates else {
            continue;
        };
        let Some(annotation) = registry.annotation(source_id) else {
            continue;
        };
        let AnnotatedItem { properties, .. } = &annotation;
        let AnnotationProperties::WrappingMiddleware { error_handler } = properties else {
            panic!("Unexpected annotation kind")
        };
        let Some(error_handler) = error_handler else {
            continue;
        };
        // The user may have used the `.error_handler` method on the `Blueprint` to overridde
        // the error handler provided by the annotation.
        if aux.fallible_id2error_handler_id.contains_key(&id) {
            // If that's the case, nothing to do here.
            continue;
        }

        let krate = krate_collection
            .get_crate_by_package_id(&source_id.package_id)
            .unwrap();

        // If that's not the case, we must process it!
        let identifiers = RawIdentifiers {
            created_at: annotation
                .created_at(krate, krate_collection.package_graph())
                .expect("Failed to determine `CreatedAt` for an annotated item"),
            created_by: CreatedBy::macro_name("wrap"),
            import_path: error_handler.to_owned(),
        };
        let identifiers_id = aux.identifiers_interner.get_or_intern(identifiers);
        let component = UserComponent::ErrorHandler {
            source: identifiers_id.into(),
            fallible_id: id,
        };
        let registration = {
            let item = krate.get_item_by_local_type_id(&source_id.rustdoc_item_id);
            Registration::annotated_item(&item, krate)
        };
        aux.intern_component(
            component,
            aux.id2scope_id[id],
            aux.id2lifecycle[id],
            registration,
        );
    }
}
