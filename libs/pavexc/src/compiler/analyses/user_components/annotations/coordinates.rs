use crate::{
    DiagnosticSink, compiler::analyses::computations::ComputationDb, rustdoc::CrateCollection,
};

use super::{
    AuxiliaryData, ImplInfo, cannot_resolve_callable_path, rustdoc_free_fn2callable,
    rustdoc_method2callable,
};

/// Resolve coordinates to the annotation they point to.
/// Then process the corresponding item.
pub(crate) fn resolve_annotation_coordinates(
    aux: &mut AuxiliaryData,
    computation_db: &mut ComputationDb,
    krate_collection: &CrateCollection,
    diagnostics: &DiagnosticSink,
) {
    for (component_id, coordinates_id) in aux.iter().filter_map(|(component_id, component)| {
        component.coordinates_id().map(|id| (component_id, id))
    }) {
        let coordinates = &aux.annotation_coordinates_interner[coordinates_id];
        let (krate, annotation) = match krate_collection.annotation_for_coordinates(coordinates) {
            Ok(Some(v)) => v,
            // TODO: diagnostics
            Ok(None) => panic!("Can't match blueprint registration to its annotation"),
            Err(e) => panic!("Can't find the package where the annotation was defined: {e:?}"),
        };

        let item = krate.get_item_by_local_type_id(&annotation.id);
        let outcome = match annotation.impl_ {
            Some(ImplInfo { attached_to, impl_ }) => {
                rustdoc_method2callable(attached_to, impl_, &item, krate, krate_collection)
            }
            None => rustdoc_free_fn2callable(&item, krate, krate_collection),
        };
        let callable = match outcome {
            Ok(callable) => callable,
            Err(e) => {
                cannot_resolve_callable_path(
                    e,
                    component_id,
                    aux,
                    krate_collection.package_graph(),
                    diagnostics,
                );
                continue;
            }
        };
        computation_db.get_or_intern_with_id(callable, component_id.into());
    }
}
