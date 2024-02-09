use crate::compiler::analyses::components::component::Component;
use crate::compiler::analyses::components::{
    ComponentDb, ComponentId, ConsumptionMode, InsertTransformer, SourceId,
};
use crate::compiler::analyses::computations::ComputationId;
use crate::compiler::analyses::user_components::{ScopeId, UserComponentId};
use crate::compiler::component::ErrorHandler;
use pavex_bp_schema::{CloningStrategy, Lifecycle};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// All the information needed to register a component.
/// Some of this information will not be attached directly to the final [`Component`],
/// but it'll end up being tracked out of band in [`ComponentDb`] (e.g. the lifecycle in
/// `ComponentDb::id2lifecycle`).
pub(crate) enum UnregisteredComponent {
    RequestHandler {
        user_component_id: UserComponentId,
    },
    UserWrappingMiddleware {
        user_component_id: UserComponentId,
    },
    SyntheticWrappingMiddleware {
        computation_id: ComputationId,
        scope_id: ScopeId,
    },
    ErrorHandler {
        source_id: SourceId,
        fallible_component_id: ComponentId,
        error_handler: ErrorHandler,
    },
    ErrorObserver {
        user_component_id: UserComponentId,
        error_input_index: usize,
    },
    UserConstructor {
        user_component_id: UserComponentId,
    },
    SyntheticConstructor {
        lifecycle: Lifecycle,
        computation_id: ComputationId,
        scope_id: ScopeId,
        cloning_strategy: CloningStrategy,
        /// Synthetic constructors can be built by "deriving" user-registered constructors.
        /// For example, by binding unassigned generic parameters or by extracting the `Ok` variant
        /// from the output of fallible constructors.
        ///
        /// If that's the case,
        /// this field should be populated with the id of the "source" constructor.
        derived_from: Option<ComponentId>,
    },
    Transformer {
        computation_id: ComputationId,
        transformed_component_id: ComponentId,
        transformation_mode: ConsumptionMode,
        scope_id: ScopeId,
        when_to_insert: InsertTransformer,
    },
}

impl UnregisteredComponent {
    /// Convert this unregistered component into a registered component **without interning it**.
    pub fn component(&self) -> Component {
        match self {
            UnregisteredComponent::RequestHandler { user_component_id } => {
                Component::RequestHandler {
                    user_component_id: user_component_id.to_owned(),
                }
            }
            UnregisteredComponent::UserWrappingMiddleware { user_component_id } => {
                Component::WrappingMiddleware {
                    source_id: SourceId::UserComponentId(user_component_id.to_owned()),
                }
            }
            UnregisteredComponent::SyntheticWrappingMiddleware {
                computation_id,
                scope_id,
            } => Component::WrappingMiddleware {
                source_id: SourceId::ComputationId(computation_id.to_owned(), *scope_id),
            },
            UnregisteredComponent::ErrorHandler { source_id, .. } => Component::ErrorHandler {
                source_id: source_id.to_owned(),
            },
            UnregisteredComponent::ErrorObserver {
                user_component_id, ..
            } => Component::ErrorObserver {
                user_component_id: user_component_id.to_owned(),
            },
            UnregisteredComponent::UserConstructor { user_component_id } => {
                Component::Constructor {
                    source_id: SourceId::UserComponentId(user_component_id.to_owned()),
                }
            }
            UnregisteredComponent::SyntheticConstructor {
                computation_id,
                scope_id,
                ..
            } => Component::Constructor {
                source_id: SourceId::ComputationId(computation_id.to_owned(), *scope_id),
            },
            UnregisteredComponent::Transformer {
                computation_id,
                transformed_component_id,
                transformation_mode,
                scope_id,
                ..
            } => Component::Transformer {
                computation_id: computation_id.to_owned(),
                transformed_component_id: transformed_component_id.to_owned(),
                transformation_mode: transformation_mode.to_owned(),
                scope_id: scope_id.to_owned(),
            },
        }
    }

    pub fn lifecycle(&self, component_db: &ComponentDb) -> Lifecycle {
        use UnregisteredComponent::*;
        match &self {
            UserWrappingMiddleware { .. }
            | SyntheticWrappingMiddleware { .. }
            | RequestHandler { .. } => Lifecycle::RequestScoped,
            ErrorObserver { .. } => Lifecycle::Transient,
            SyntheticConstructor { lifecycle, .. } => lifecycle.to_owned(),
            ErrorHandler {
                fallible_component_id: id,
                ..
            }
            | Transformer {
                transformed_component_id: id,
                ..
            } => component_db.lifecycle(*id),
            UserConstructor {
                user_component_id, ..
            } => component_db
                .user_component_db()
                .get_lifecycle(*user_component_id)
                .to_owned(),
        }
    }
}
