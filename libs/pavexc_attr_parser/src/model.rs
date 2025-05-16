use darling::util::Ignored;

use crate::AnnotationProperties;

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

impl From<ConstructorProperties> for AnnotationProperties {
    fn from(value: ConstructorProperties) -> Self {
        AnnotationProperties::Constructor {
            lifecycle: value.lifecycle.into(),
            cloning_strategy: value.cloning_strategy.map(Into::into),
            error_handler: value.error_handler,
        }
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// The way we expect wrapping middleware properties to be represented in
/// `pavex::diagnostic::wrap`.
pub struct WrappingMiddlewareProperties {
    pub error_handler: Option<String>,
    pub id: Ignored,
}

impl From<WrappingMiddlewareProperties> for AnnotationProperties {
    fn from(value: WrappingMiddlewareProperties) -> Self {
        AnnotationProperties::WrappingMiddleware {
            error_handler: value.error_handler,
        }
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// The way we expect pre-processing middleware properties to be represented in
/// `pavex::diagnostic::pre_process`.
pub struct PreProcessingMiddlewareProperties {
    pub error_handler: Option<String>,
    pub id: Ignored,
}

impl From<PreProcessingMiddlewareProperties> for AnnotationProperties {
    fn from(value: PreProcessingMiddlewareProperties) -> Self {
        AnnotationProperties::PreProcessingMiddleware {
            error_handler: value.error_handler,
        }
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// The way we expect post-processing middleware properties to be represented in
/// `pavex::diagnostic::post_process`.
pub struct PostProcessingMiddlewareProperties {
    pub error_handler: Option<String>,
    pub id: Ignored,
}

impl From<PostProcessingMiddlewareProperties> for AnnotationProperties {
    fn from(value: PostProcessingMiddlewareProperties) -> Self {
        AnnotationProperties::PostProcessingMiddleware {
            error_handler: value.error_handler,
        }
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// The way we expect config properties to be represented in
/// `pavex::diagnostic::config`.
///
/// It is a more verbose (but easier to parse) representation than
/// what is used by `pavex::config`.
pub struct ConfigProperties {
    pub key: String,
    pub cloning_strategy: Option<CloningStrategy>,
    pub default_if_missing: Option<bool>,
    pub include_if_unused: Option<bool>,
}

impl From<ConfigProperties> for AnnotationProperties {
    fn from(value: ConfigProperties) -> Self {
        AnnotationProperties::Config {
            key: value.key,
            cloning_strategy: value.cloning_strategy.map(Into::into),
            default_if_missing: value.default_if_missing,
            include_if_unused: value.include_if_unused,
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
