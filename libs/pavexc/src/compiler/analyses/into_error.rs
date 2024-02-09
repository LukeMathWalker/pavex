use std::borrow::Cow;

use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::components::{ConsumptionMode, InsertTransformer};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::user_components::ScopeId;
use crate::compiler::computation::{Computation, MatchResultVariant};
use crate::language::{Callable, InvocationStyle, PathType, ResolvedPath, ResolvedPathSegment};

/// Returns the [`ComponentId`] for a transformer component that calls `pavex::Error::new` on the
/// error returned by a fallible computation.
///
/// If the component is not an error matcher, it returns `None`.
pub(super) fn register_error_new_transformer(
    component_id: ComponentId,
    component_db: &mut ComponentDb,
    computation_db: &mut ComputationDb,
    scope_id: ScopeId,
    pavex_error: &PathType,
) -> Option<ComponentId> {
    let Computation::MatchResult(m) = component_db
        .hydrated_component(component_id, computation_db)
        .computation()
    else {
        return None;
    };
    if m.variant != MatchResultVariant::Err {
        return None;
    }
    let error = m.output.clone();

    let pavex_error_path = &pavex_error.resolved_path();
    let pavex_error_new_segments = {
        let mut c = pavex_error_path.segments.clone();
        c.push(ResolvedPathSegment {
            ident: "new".into(),
            generic_arguments: vec![],
        });
        c
    };
    let pavex_error_new_path = ResolvedPath {
        segments: pavex_error_new_segments,
        qualified_self: None,
        package_id: pavex_error.package_id.clone(),
    };

    let pavex_error_new_callable = Callable {
        is_async: false,
        takes_self_as_ref: true,
        output: Some(pavex_error.clone().into()),
        path: pavex_error_new_path,
        inputs: vec![error.to_owned()],
        invocation_style: InvocationStyle::FunctionCall,
        source_coordinates: None,
    };

    let computation_id =
        computation_db.get_or_intern(Computation::Callable(Cow::Owned(pavex_error_new_callable)));
    let pavex_error_new_component_id = component_db.get_or_intern_transformer(
        computation_id,
        component_id,
        scope_id,
        InsertTransformer::Eagerly,
        ConsumptionMode::Move,
        computation_db,
    );
    Some(pavex_error_new_component_id)
}
