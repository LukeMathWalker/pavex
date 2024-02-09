use crate::compiler::analyses::components::ComponentId;
use crate::compiler::analyses::components::{ConsumptionMode, SourceId};
use crate::compiler::analyses::computations::ComputationId;
use crate::compiler::analyses::user_components::{ScopeId, UserComponentId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// The item stored in [`ComponentDb`].
pub(crate) enum Component {
    RequestHandler {
        user_component_id: UserComponentId,
    },
    WrappingMiddleware {
        source_id: SourceId,
    },
    ErrorHandler {
        source_id: SourceId,
    },
    ErrorObserver {
        user_component_id: UserComponentId,
    },
    Constructor {
        source_id: SourceId,
    },
    Transformer {
        computation_id: ComputationId,
        transformed_component_id: ComponentId,
        transformation_mode: ConsumptionMode,
        scope_id: ScopeId,
    },
}
