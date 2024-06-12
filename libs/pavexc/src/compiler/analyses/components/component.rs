use crate::compiler::analyses::components::{ComponentId, InsertTransformer};
use crate::compiler::analyses::components::{ConsumptionMode, SourceId};
use crate::compiler::analyses::user_components::UserComponentId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// The item stored in [`ComponentDb`].
pub(crate) enum Component {
    RequestHandler {
        user_component_id: UserComponentId,
    },
    PrebuiltType {
        user_component_id: UserComponentId,
    },
    WrappingMiddleware {
        source_id: SourceId,
    },
    PreProcessingMiddleware {
        source_id: SourceId,
    },
    PostProcessingMiddleware {
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

impl Component {
    /// Get the source id of this component.
    pub(crate) fn source_id(&self) -> SourceId {
        match self {
            Component::PrebuiltType { user_component_id }
            | Component::RequestHandler { user_component_id } => user_component_id.clone().into(),
            Component::WrappingMiddleware { source_id }
            | Component::PostProcessingMiddleware { source_id }
            | Component::PreProcessingMiddleware { source_id }
            | Component::Constructor { source_id }
            | Component::Transformer { source_id, .. } => source_id.clone(),
            Component::ErrorObserver { user_component_id } => user_component_id.clone().into(),
        }
    }
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
