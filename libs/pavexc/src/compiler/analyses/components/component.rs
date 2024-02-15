use crate::compiler::analyses::components::{ComponentId, InsertTransformer};
use crate::compiler::analyses::components::{ConsumptionMode, SourceId};
use crate::compiler::analyses::user_components::UserComponentId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// The item stored in [`ComponentDb`].
pub(crate) enum Component {
    RequestHandler {
        user_component_id: UserComponentId,
    },
    WrappingMiddleware {
        source_id: SourceId,
    },
    ErrorObserver {
        user_component_id: UserComponentId,
    },
    Constructor {
        source_id: SourceId,
    },
    Transformer {
        source_id: SourceId,
        transformed_component_id: ComponentId,
    },
}

/// Additional information about a transformer in [`ComponentDb`].
/// This information is not necessary to determine the "identity" of a transformer,
/// therefore it is not stored inside [`Component::Transformer`] to keep the size of the enum small.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TransformerInfo {
    /// A transformer must apply some kind of computation to the output of
    /// the component it is transforming.
    /// This field tracks the index of the transformer's input parameter
    /// that corresponds to the output of the transformed component.
    pub(crate) input_index: usize,
    /// Determine if the transformer should be inserted in the graph eagerly or lazily.
    pub(crate) when_to_insert: InsertTransformer,
    /// Determine if the transformed type should be taken by value or by reference.
    pub(crate) transformation_mode: ConsumptionMode,
}
