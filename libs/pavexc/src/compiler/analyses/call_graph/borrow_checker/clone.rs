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
use crate::compiler::utils::resolve_type_path;
use crate::language::{
    Callable, FQPath, FQPathSegment, FQQualifiedSelf, InvocationStyle, Lifetime, PathType,
    ResolvedType, TypeReference,
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
        let ResolvedType::ResolvedPath(clone) = clone else {
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

    let clone_path = clone.resolved_path();
    let clone_segments = {
        let mut c = clone_path.segments.clone();
        c.push(FQPathSegment {
            ident: "clone".into(),
            generic_arguments: vec![],
        });
        c
    };
    let type_clone_path = FQPath {
        segments: clone_segments,
        qualified_self: Some(FQQualifiedSelf {
            position: clone_path.segments.len(),
            type_: output.clone().into(),
        }),
        package_id: clone_path.package_id,
    };

    let clone_callable = Callable {
        is_async: false,
        takes_self_as_ref: true,
        output: Some(output.clone()),
        path: type_clone_path,
        inputs: vec![ResolvedType::Reference(TypeReference {
            is_mutable: false,
            lifetime: Lifetime::Elided,
            inner: Box::new(output),
        })],
        invocation_style: InvocationStyle::FunctionCall,
        source_coordinates: None,
    };

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
