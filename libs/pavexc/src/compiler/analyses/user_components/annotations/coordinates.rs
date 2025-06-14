use crate::{
    DiagnosticSink, compiler::analyses::computations::ComputationDb, rustdoc::CrateCollection,
};
use pavexc_attr_parser::AnnotationProperties;

use super::{
    AuxiliaryData, ImplInfo, cannot_resolve_callable_path, rustdoc_free_fn2callable,
    rustdoc_method2callable, validate_route_path,
};
use crate::compiler::analyses::user_components::UserComponent;

/// Resolve coordinates to the annotation they point to.
/// Then process the corresponding item.
pub(crate) fn resolve_annotation_coordinates(
    aux: &mut AuxiliaryData,
    computation_db: &mut ComputationDb,
    krate_collection: &CrateCollection,
    diagnostics: &DiagnosticSink,
) {
    // Collect all components with coordinates first to avoid borrow checker issues
    let components_with_coords: Vec<_> = aux
        .iter()
        .filter_map(|(component_id, component)| {
            component.coordinates_id().map(|id| (component_id, id))
        })
        .collect();

    for (component_id, coordinates_id) in components_with_coords {
        let coordinates = &aux.annotation_coordinates_interner[coordinates_id];
        let (krate, annotation) = match krate_collection.annotation_for_coordinates(coordinates) {
            Ok(Ok(Some(v))) => v,
            // TODO: diagnostics
            Ok(Ok(None)) => panic!("Can't match blueprint registration to its annotation"),
            // TODO: diagnostics
            Ok(Err(e)) => panic!("Can't find the package where the annotation was defined: {e:?}"),
            Err(_) => {
                // A diagnostic has already been emitted.
                continue;
            }
        };

        let item = krate.get_item_by_local_type_id(&annotation.id);

        // Retrieve routing information for routes that have been registered directly against the blueprint,
        // rather than via an import.
        if let AnnotationProperties::Route { method, path, .. } = &annotation.properties {
            if matches!(
                aux.component_interner[component_id],
                UserComponent::RequestHandler { .. }
            ) {
                validate_route_path(aux, component_id, path, diagnostics);

                let UserComponent::RequestHandler { router_key, .. } =
                    &mut aux.component_interner[component_id]
                else {
                    unreachable!()
                };
                router_key.path = format!("{}{}", router_key.path, path);
                router_key.method_guard = method.clone();
            }
        }

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
