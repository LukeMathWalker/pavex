use pavex_bp_schema::MethodGuard;

use crate::{AnnotationProperties, atoms::MethodArgument};

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// The way we expect constructor properties to be represented in
/// `pavex::diagnostic::constructor`.
///
/// It is a more verbose (but easier to parse) representation than
/// what is used by `pavex::constructor`.
pub struct ConstructorProperties {
    pub id: String,
    pub lifecycle: Lifecycle,
    pub cloning_policy: Option<CloningPolicy>,
    pub allow_unused: Option<bool>,
    pub allow_error_fallback: Option<bool>,
}

impl From<ConstructorProperties> for AnnotationProperties {
    fn from(value: ConstructorProperties) -> Self {
        AnnotationProperties::Constructor {
            id: value.id,
            lifecycle: value.lifecycle.into(),
            cloning_policy: value.cloning_policy.map(Into::into),
            allow_unused: value.allow_unused,
            allow_error_fallback: value.allow_error_fallback,
        }
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// The way we expect error observer properties to be represented in
/// `pavex::diagnostic::error_observer`.
pub struct ErrorObserverProperties {
    pub id: String,
}

impl From<ErrorObserverProperties> for AnnotationProperties {
    fn from(value: ErrorObserverProperties) -> Self {
        AnnotationProperties::ErrorObserver { id: value.id }
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// The way we expect error handler properties to be represented in
/// `pavex::diagnostic::error_handler`.
pub struct ErrorHandlerProperties {
    pub id: String,
    pub error_ref_input_index: usize,
    pub default: Option<bool>,
}

impl From<ErrorHandlerProperties> for AnnotationProperties {
    fn from(value: ErrorHandlerProperties) -> Self {
        AnnotationProperties::ErrorHandler {
            error_ref_input_index: value.error_ref_input_index,
            default: value.default,
            id: value.id,
        }
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// The way we expect wrapping middleware properties to be represented in
/// `pavex::diagnostic::wrap`.
pub struct WrappingMiddlewareProperties {
    pub id: String,
    pub allow_error_fallback: Option<bool>,
}

impl From<WrappingMiddlewareProperties> for AnnotationProperties {
    fn from(value: WrappingMiddlewareProperties) -> Self {
        AnnotationProperties::WrappingMiddleware {
            id: value.id,
            allow_error_fallback: value.allow_error_fallback,
        }
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// The way we expect pre-processing middleware properties to be represented in
/// `pavex::diagnostic::pre_process`.
pub struct PreProcessingMiddlewareProperties {
    pub id: String,
    pub allow_error_fallback: Option<bool>,
}

impl From<PreProcessingMiddlewareProperties> for AnnotationProperties {
    fn from(value: PreProcessingMiddlewareProperties) -> Self {
        AnnotationProperties::PreProcessingMiddleware {
            id: value.id,
            allow_error_fallback: value.allow_error_fallback,
        }
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// The way we expect post-processing middleware properties to be represented in
/// `pavex::diagnostic::post_process`.
pub struct PostProcessingMiddlewareProperties {
    pub id: String,
    pub allow_error_fallback: Option<bool>,
}

impl From<PostProcessingMiddlewareProperties> for AnnotationProperties {
    fn from(value: PostProcessingMiddlewareProperties) -> Self {
        AnnotationProperties::PostProcessingMiddleware {
            id: value.id,
            allow_error_fallback: value.allow_error_fallback,
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
    pub id: String,
    pub key: String,
    pub cloning_policy: Option<CloningPolicy>,
    pub default_if_missing: Option<bool>,
    pub include_if_unused: Option<bool>,
}

impl From<ConfigProperties> for AnnotationProperties {
    fn from(value: ConfigProperties) -> Self {
        AnnotationProperties::Config {
            id: value.id,
            key: value.key,
            cloning_policy: value.cloning_policy.map(Into::into),
            default_if_missing: value.default_if_missing,
            include_if_unused: value.include_if_unused,
        }
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// The way we expect fallback properties to be represented in
/// `pavex::diagnostic::fallback`.
pub struct FallbackProperties {
    pub id: String,
    pub allow_error_fallback: Option<bool>,
}

impl From<FallbackProperties> for AnnotationProperties {
    fn from(value: FallbackProperties) -> Self {
        AnnotationProperties::Fallback {
            id: value.id,
            allow_error_fallback: value.allow_error_fallback,
        }
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// The way we expect route properties to be represented in
/// `pavex::diagnostic::route`.
pub struct RouteProperties {
    pub id: String,
    pub path: String,
    pub method: Option<MethodArgument>,
    pub allow_any_method: Option<bool>,
    pub allow_non_standard_methods: Option<bool>,
    pub allow_error_fallback: Option<bool>,
}

impl From<RouteProperties> for AnnotationProperties {
    fn from(value: RouteProperties) -> Self {
        let method = match value.method {
            Some(m) => m.into(),
            None => {
                if value.allow_any_method != Some(true) {
                    panic!(
                        "Malformed `pavex::diagnostic::route` attribute. You must either accept a list of given methods or allow any method to pass through."
                    );
                }
                if value.allow_non_standard_methods == Some(true) {
                    MethodGuard::Any
                } else {
                    MethodGuard::Some(
                        [
                            "CONNECT", "GET", "POST", "PUT", "DELETE", "PATCH", "OPTIONS", "HEAD",
                            "TRACE",
                        ]
                        .iter()
                        .map(|&method| method.to_string())
                        .collect(),
                    )
                }
            }
        };
        AnnotationProperties::Route {
            id: value.id,
            path: value.path,
            method,
            allow_error_fallback: value.allow_error_fallback,
        }
    }
}

#[derive(darling::FromMeta, Debug, Clone, PartialEq, Eq)]
/// The way we expect prebuilt properties to be represented in
/// `pavex::diagnostic::prebuilt`.
pub struct PrebuiltProperties {
    pub id: String,
    pub cloning_policy: Option<CloningPolicy>,
}

impl From<PrebuiltProperties> for AnnotationProperties {
    fn from(value: PrebuiltProperties) -> Self {
        AnnotationProperties::Prebuilt {
            id: value.id,
            cloning_policy: value.cloning_policy.map(Into::into),
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
pub enum CloningPolicy {
    CloneIfNecessary,
    NeverClone,
}

impl From<CloningPolicy> for pavex_bp_schema::CloningPolicy {
    fn from(value: CloningPolicy) -> Self {
        match value {
            CloningPolicy::CloneIfNecessary => pavex_bp_schema::CloningPolicy::CloneIfNecessary,
            CloningPolicy::NeverClone => pavex_bp_schema::CloningPolicy::NeverClone,
        }
    }
}
