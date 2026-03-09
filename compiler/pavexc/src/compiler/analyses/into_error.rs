use std::borrow::Cow;

use crate::compiler::analyses::components::{ComponentDb, ComponentId};
use crate::compiler::analyses::components::{ConsumptionMode, InsertTransformer};
use crate::compiler::analyses::computations::ComputationDb;
use crate::compiler::analyses::user_components::ScopeId;
use crate::compiler::computation::{Computation, MatchResultVariant};
use crate::language::{
    Callable, CallableInput, CallableMetadata, FnHeader, InherentMethod, InherentMethodPath,
    ParameterName, Type,
};

/// Returns the [`ComponentId`] for a transformer component that calls `pavex::Error::new` on the
/// error returned by a fallible computation.
///
/// If the component is not an error matcher, it returns `None`.
pub(super) fn register_error_new_transformer(
    component_id: ComponentId,
    component_db: &mut ComponentDb,
    computation_db: &mut ComputationDb,
    scope_id: ScopeId,
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

    let Type::Path(pavex_error) = &component_db.pavex_error else {
        unreachable!()
    };
    let crate_name = pavex_error.base_type.first().cloned().unwrap_or_default();
    let type_name = pavex_error.base_type.last().cloned().unwrap_or_default();
    let module_path = if pavex_error.base_type.len() > 2 {
        pavex_error.base_type[1..pavex_error.base_type.len() - 1].to_vec()
    } else {
        vec![]
    };

    let pavex_error_new_callable = Callable::InherentMethod(InherentMethod {
        path: InherentMethodPath {
            package_id: pavex_error.package_id.clone(),
            crate_name,
            module_path,
            type_name,
            type_generics: vec![],
            method_name: "new".into(),
            method_generics: vec![],
        },
        metadata: CallableMetadata {
            output: Some(pavex_error.clone().into()),
            inputs: vec![CallableInput {
                name: ParameterName::new("_0".into()),
                type_: error.to_owned(),
            }],
            source_coordinates: None,
        },
        header: FnHeader {
            is_async: false,
            abi: rustdoc_types::Abi::Rust,
            is_unsafe: false,
            is_c_variadic: false,
            symbol_name: None,
        },
        takes_self_as_ref: true,
    });

    let computation_id =
        computation_db.get_or_intern(Computation::Callable(Cow::Owned(pavex_error_new_callable)));
    let pavex_error_new_component_id = component_db.get_or_intern_transformer(
        computation_id,
        component_id,
        scope_id,
        InsertTransformer::Eagerly,
        ConsumptionMode::Move,
        0,
        computation_db,
    );
    Some(pavex_error_new_component_id)
}
