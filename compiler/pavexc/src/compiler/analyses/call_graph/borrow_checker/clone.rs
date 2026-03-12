use std::borrow::Cow;

use once_cell::sync::OnceCell;

use pavex_bp_schema::CloningPolicy;

use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::components::{
    ConsumptionMode, HydratedComponent, InsertTransformer,
};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::user_components::ScopeId;
use crate::compiler::computation::Computation;
use crate::compiler::framework_rustdoc::resolve_type_path;
use crate::language::{
    Callable, CallableInput, FnHeader, Lifetime, PathType, RustIdentifier, TraitMethod,
    TraitMethodPath, Type, TypeReference,
};
use crate::rustdoc::CrateCollection;

/// Returns the [`ComponentId`] for a transformer component that calls `Clone::clone` on the
/// computation underpinning the given `component_id`.
///
/// If the component is not a constructor, it returns `None`.
pub(super) fn get_clone_component_id(
    component_id: &ComponentId,
    krate_collection: &CrateCollection,
    component_db: &mut ComponentDb,
    computation_db: &mut ComputationDb,
    scope_id: ScopeId,
) -> Option<ComponentId> {
    // We only need to resolve this once.
    static CLONE_PATH_TYPE: OnceCell<PathType> = OnceCell::new();
    let clone = CLONE_PATH_TYPE.get_or_init(|| {
        let clone = resolve_type_path("std::clone::Clone", krate_collection);
        let Type::Path(clone) = clone else {
            unreachable!()
        };
        clone
    });

    let HydratedComponent::Constructor(c) =
        component_db.hydrated_component(*component_id, computation_db)
    else {
        return None;
    };
    let output = c.output_type().to_owned();

    // We only add a cloning node if the component is not marked as `NeverClone`.
    let cloning_policy = component_db.cloning_policy(*component_id);
    if cloning_policy == CloningPolicy::NeverClone {
        return None;
    }

    let crate_name = clone.base_type.first().cloned().unwrap_or_default();
    let trait_name = clone.base_type.last().cloned().unwrap_or_default();
    let module_path = if clone.base_type.len() > 2 {
        clone.base_type[1..clone.base_type.len() - 1].to_vec()
    } else {
        vec![]
    };

    let clone_callable = Callable::TraitMethod(TraitMethod {
        path: TraitMethodPath {
            package_id: clone.package_id.clone(),
            crate_name,
            module_path,
            trait_name,
            trait_generics: vec![],
            self_type: output.clone(),
            method_name: "clone".into(),
            method_generics: vec![],
        },
        header: FnHeader {
            output: Some(output.clone()),
            inputs: vec![CallableInput {
                name: RustIdentifier::new("_0".into()),
                type_: Type::Reference(TypeReference {
                    is_mutable: false,
                    lifetime: Lifetime::Elided,
                    inner: Box::new(output),
                }),
            }],
            is_async: false,
            abi: rustdoc_types::Abi::Rust,
            is_unsafe: false,
            is_c_variadic: false,
            symbol_name: None,
        },
        source_coordinates: None,
        takes_self_as_ref: true,
    });

    let clone_computation_id =
        computation_db.get_or_intern(Computation::Callable(Cow::Owned(clone_callable)));
    let clone_component_id = component_db.get_or_intern_transformer(
        clone_computation_id,
        *component_id,
        scope_id,
        InsertTransformer::Lazily,
        ConsumptionMode::SharedBorrow,
        0,
        computation_db,
    );
    Some(clone_component_id)
}
