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
    UserPostProcessingMiddleware {
        user_component_id: UserComponentId,
    },
    UserPreProcessingMiddleware {
        user_component_id: UserComponentId,
    },
    UserPrebuiltType {
        user_component_id: UserComponentId,
    },
    SyntheticWrappingMiddleware {
        computation_id: ComputationId,
        scope_id: ScopeId,
        /// Synthetic middlewares are usually created by binding the generic parameter for Next,
        /// except for the noop middleware.
        derived_from: Option<ComponentId>,
    },
    ErrorHandler {
        source_id: SourceId,
        error_matcher_id: ComponentId,
        /// The id of the component that returns the error type that this error handler handles.
        ///
        /// It can be a fallible component (e.g. constructor or request handler) if the error type
        /// returned by the fallible component is the same as the error type taken as input by the error handler.
        ///
        /// It can be a transformer if the error type is transformed
        /// before being handled by the error handler—i.e. if it gets upcasted to a `pavex::Error` type.
        error_source_id: ComponentId,
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
        /// The index of the input parameter of the transformer that corresponds to the output of the
        /// transformed component.
        transformed_input_index: usize,
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
                ..
            } => Component::WrappingMiddleware {
                source_id: SourceId::ComputationId(computation_id.to_owned(), *scope_id),
            },
            UnregisteredComponent::ErrorHandler {
                source_id,
                error_source_id,
                ..
            } => Component::Transformer {
                source_id: source_id.to_owned(),
                transformed_component_id: *error_source_id,
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
                scope_id,
                ..
            } => Component::Transformer {
                source_id: SourceId::ComputationId(computation_id.to_owned(), *scope_id),
                transformed_component_id: transformed_component_id.to_owned(),
            },
            UnregisteredComponent::UserPostProcessingMiddleware { user_component_id } => {
                Component::PostProcessingMiddleware {
                    source_id: SourceId::UserComponentId(user_component_id.to_owned()),
                }
            }
            UnregisteredComponent::UserPreProcessingMiddleware { user_component_id } => {
                Component::PreProcessingMiddleware {
                    source_id: SourceId::UserComponentId(user_component_id.to_owned()),
                }
            }
            UnregisteredComponent::UserPrebuiltType { user_component_id } => {
                Component::PrebuiltType {
                    user_component_id: *user_component_id,
                }
            }
        }
    }

    pub fn lifecycle(&self, component_db: &ComponentDb) -> Lifecycle {
        use UnregisteredComponent::*;
        match &self {
            UserWrappingMiddleware { .. }
            | UserPostProcessingMiddleware { .. }
            | UserPreProcessingMiddleware { .. }
            | SyntheticWrappingMiddleware { .. }
            | RequestHandler { .. } => Lifecycle::RequestScoped,
            ErrorObserver { .. } => Lifecycle::Transient,
            UserPrebuiltType { .. } => Lifecycle::Singleton,
            SyntheticConstructor { lifecycle, .. } => lifecycle.to_owned(),
            ErrorHandler {
                error_matcher_id: id,
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
