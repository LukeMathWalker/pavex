use crate::AnnotatedComponent;

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// The way we expect constructor properties to be represented in
/// `pavex::diagnostic::constructor`.
///
/// It is a more verbose (but easier to parse) representation than
/// what is used by `pavex::constructor`.
pub struct ConstructorProperties {
    pub lifecycle: Lifecycle,
    pub cloning_strategy: Option<CloningStrategy>,
    pub error_handler: Option<String>,
}

impl From<ConstructorProperties> for AnnotatedComponent {
    fn from(value: ConstructorProperties) -> Self {
        AnnotatedComponent::Constructor {
            lifecycle: value.lifecycle.into(),
            cloning_strategy: value.cloning_strategy.map(Into::into),
            error_handler: value.error_handler,
        }
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
#[darling(rename_all = "snake_case")]
pub enum Lifecycle {
    Singleton,
    RequestScoped,
    Transient,
}

impl From<Lifecycle> for pavex_bp_schema::Lifecycle {
    fn from(value: Lifecycle) -> Self {
        match value {
            Lifecycle::Singleton => pavex_bp_schema::Lifecycle::Singleton,
            Lifecycle::RequestScoped => pavex_bp_schema::Lifecycle::RequestScoped,
            Lifecycle::Transient => pavex_bp_schema::Lifecycle::Transient,
        }
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
#[darling(rename_all = "snake_case")]
pub enum CloningStrategy {
    CloneIfNecessary,
    NeverClone,
}

impl From<CloningStrategy> for pavex_bp_schema::CloningStrategy {
    fn from(value: CloningStrategy) -> Self {
        match value {
            CloningStrategy::CloneIfNecessary => pavex_bp_schema::CloningStrategy::CloneIfNecessary,
            CloningStrategy::NeverClone => pavex_bp_schema::CloningStrategy::NeverClone,
        }
    }
}
