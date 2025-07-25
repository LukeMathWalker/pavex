use super::diagnostic::invalid_prebuilt_type;
use crate::compiler::analyses::prebuilt_types::PrebuiltTypeDb;
use crate::{
    DiagnosticSink, compiler::analyses::computations::ComputationDb, rustdoc::CrateCollection,
};
use pavexc_attr_parser::AnnotationProperties;

use super::{
    AuxiliaryData, ConfigType, FQPath, FQPathSegment, ImplInfo, annotated_item2type,
    cannot_resolve_callable_path, invalid_config_type, rustdoc_free_fn2callable,
    rustdoc_method2callable, validate_route_path,
};
use crate::compiler::analyses::user_components::{UserComponent, UserComponentId};
use crate::compiler::component::{DefaultStrategy, PrebuiltType};
use pavex_bp_schema::{CloningPolicy, Lint, LintSetting};

/// Resolve coordinates to the annotation they point to.
/// Then process the corresponding item.
pub(crate) fn resolve_annotation_coordinates(
    aux: &mut AuxiliaryData,
    computation_db: &mut ComputationDb,
    prebuilt_type_db: &mut PrebuiltTypeDb,
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
        if let AnnotationProperties::Route {
            method,
            path,
            // Handled below.
            allow_error_fallback: _,
            id: _,
        } = &annotation.properties
        {
            validate_route_path(aux, component_id, path, diagnostics);
            let UserComponent::RequestHandler { router_key, .. } =
                &mut aux.component_interner[component_id]
            else {
                unreachable!()
            };
            router_key.path = format!("{}{}", router_key.path, path);
            router_key.method_guard = method.clone();
        }

        // Retrieve config properties for config types that have been registered directly against the blueprint
        if let AnnotationProperties::Config {
            key,
            cloning_policy,
            default_if_missing,
            include_if_unused,
            id: _,
        } = &annotation.properties
        {
            let UserComponent::ConfigType {
                key: config_key, ..
            } = &mut aux.component_interner[component_id]
            else {
                unreachable!()
            };
            *config_key = key.clone();
            // Use the behaviour specified in the annotation, unless the user has overridden
            // it when registering the config directly with the blueprint.
            aux.id2cloning_policy
                .entry(component_id)
                .or_insert_with(|| cloning_policy.unwrap_or(CloningPolicy::CloneIfNecessary));
            aux.config_id2default_strategy
                .entry(component_id)
                .or_insert_with(|| {
                    default_if_missing
                        .map(|b| {
                            if b {
                                DefaultStrategy::DefaultIfMissing
                            } else {
                                DefaultStrategy::Required
                            }
                        })
                        .unwrap_or_default()
                });
            aux.config_id2include_if_unused
                .entry(component_id)
                .or_insert_with(|| include_if_unused.unwrap_or(false));

            let Ok(ty) = annotated_item2type(&item, krate, krate_collection, diagnostics) else {
                continue;
            };
            match ConfigType::new(ty, config_key.to_owned()) {
                Ok(config) => {
                    aux.config_id2type.insert(component_id, config);
                }
                Err(e) => {
                    let path = FQPath {
                        segments: krate.import_index.items[&item.id]
                            .canonical_path()
                            .iter()
                            .cloned()
                            .map(FQPathSegment::new)
                            .collect(),
                        qualified_self: None,
                        package_id: krate.core.package_id.clone(),
                    };
                    invalid_config_type(e, &path, component_id, aux, diagnostics)
                }
            };
        }

        // Retrieve prebuilt properties for prebuilt types that have been registered directly against the blueprint
        if let AnnotationProperties::Prebuilt { cloning_policy, .. } = &annotation.properties {
            assert!(matches!(
                aux.component_interner[component_id],
                UserComponent::PrebuiltType { .. }
            ));

            // Use the behaviour specified in the annotation, unless the user has overridden
            // it when registering the prebuilt directly with the blueprint.
            aux.id2cloning_policy
                .entry(component_id)
                .or_insert_with(|| cloning_policy.unwrap_or(CloningPolicy::NeverClone));

            let Ok(ty) = annotated_item2type(&item, krate, krate_collection, diagnostics) else {
                continue;
            };
            match PrebuiltType::new(ty) {
                Ok(prebuilt) => {
                    prebuilt_type_db.get_or_intern(prebuilt, component_id);
                }
                Err(e) => {
                    let path = FQPath {
                        segments: krate.import_index.items[&item.id]
                            .canonical_path()
                            .iter()
                            .cloned()
                            .map(FQPathSegment::new)
                            .collect(),
                        qualified_self: None,
                        package_id: krate.core.package_id.clone(),
                    };
                    invalid_prebuilt_type(e, &path, component_id, aux, diagnostics)
                }
            };
        }

        // Retrieve constructor properties for constructors that have been registered directly against the blueprint
        if let AnnotationProperties::Constructor {
            lifecycle,
            cloning_policy,
            allow_unused,
            // Handled below.
            allow_error_fallback: _,
            id: _,
        } = &annotation.properties
        {
            assert!(matches!(
                aux.component_interner[component_id],
                UserComponent::Constructor { .. }
            ));

            // Use the lifecycle specified via the annotation, unless the user has explicitly
            // overridden it when registering the constructor directly with the blueprint.
            if !aux.id2lifecycle.contains_idx(component_id) {
                aux.id2lifecycle.insert(component_id, *lifecycle);
                if let Some(error_handler_id) = aux.fallible_id2error_handler_id.get(&component_id)
                {
                    aux.id2lifecycle.insert(*error_handler_id, *lifecycle);
                }
            }

            let lints = aux.id2lints.entry(component_id).or_default();
            if let Some(true) = allow_unused {
                lints.entry(Lint::Unused).or_insert(LintSetting::Allow);
            }

            // Use the behaviour specified in the annotation, unless the user has overridden
            // it when registering the constructor directly with the blueprint.
            aux.id2cloning_policy
                .entry(component_id)
                .or_insert_with(|| cloning_policy.unwrap_or(CloningPolicy::NeverClone));
        }

        allow_error_fallback(&annotation.properties, component_id, aux);

        if matches!(
            annotation.properties,
            AnnotationProperties::Config { .. } | AnnotationProperties::Prebuilt { .. }
        ) {
            continue;
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

/// Register if the component is allowed to fallback to the default error handler on the error path.
fn allow_error_fallback(
    properties: &AnnotationProperties,
    component_id: UserComponentId,
    aux: &mut AuxiliaryData,
) {
    match properties {
        AnnotationProperties::Constructor {
            allow_error_fallback,
            ..
        }
        | AnnotationProperties::WrappingMiddleware {
            allow_error_fallback,
            ..
        }
        | AnnotationProperties::PreProcessingMiddleware {
            allow_error_fallback,
            ..
        }
        | AnnotationProperties::PostProcessingMiddleware {
            allow_error_fallback,
            ..
        }
        | AnnotationProperties::Route {
            allow_error_fallback,
            ..
        }
        | AnnotationProperties::Fallback {
            allow_error_fallback,
            ..
        } => {
            let lints = aux.id2lints.entry(component_id).or_default();
            if let Some(true) = allow_error_fallback {
                lints
                    .entry(Lint::ErrorFallback)
                    .or_insert(LintSetting::Allow);
            }
        }
        AnnotationProperties::ErrorObserver { .. }
        | AnnotationProperties::ErrorHandler { .. }
        | AnnotationProperties::Config { .. }
        | AnnotationProperties::Methods
        | AnnotationProperties::Prebuilt { .. } => {}
    }
}
