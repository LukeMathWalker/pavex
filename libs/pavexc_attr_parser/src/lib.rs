use darling::FromMeta;
use errors::InvalidAttributeParams;
use pavex_bp_schema::{CloningStrategy, Lifecycle, MethodGuard};

pub mod atoms;
pub mod errors;
pub mod model;

/// Parse a raw `pavex` diagnostic attribute into the specification for a Pavex component.
///
/// It returns `None` for:
/// - attributes that don't belong to the `diagnostic::pavex` namespace (e.g. `#[inline]`)
/// - attributes that don't parse successfully into `syn::Attribute`
pub fn parse(
    attrs: &[String],
) -> Result<Option<AnnotationProperties>, errors::AttributeParserError> {
    let mut component = None;
    let attrs = attrs
        .iter()
        .filter_map(|a| match parse_outer_attrs(a) {
            Ok(attrs) => Some(attrs.into_iter()),
            Err(_) => None,
        })
        .flatten();
    for attr in attrs {
        let Some(sub_path) = strip_pavex_path_prefix(attr.path()) else {
            continue;
        };
        let Some(component_kind) = sub_path.get_ident() else {
            return Err(errors::UnknownPavexAttribute::new(attr.path()).into());
        };
        let Ok(kind) = AnnotationKind::parse(component_kind) else {
            return Err(errors::UnknownPavexAttribute::new(attr.path()).into());
        };
        let c = AnnotationProperties::from_meta(kind, &attr.meta)?;
        if component.is_some() {
            return Err(errors::AttributeParserError::MultiplePavexAttributes);
        } else {
            component = Some(c);
        }
    }
    Ok(component)
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AnnotationProperties {
    Constructor {
        lifecycle: Lifecycle,
        cloning_strategy: Option<CloningStrategy>,
    },
    Prebuilt {
        cloning_strategy: Option<CloningStrategy>,
    },
    Config {
        key: String,
        cloning_strategy: Option<CloningStrategy>,
        default_if_missing: Option<bool>,
        include_if_unused: Option<bool>,
    },
    WrappingMiddleware {
        id: String,
    },
    PreProcessingMiddleware {
        id: String,
    },
    PostProcessingMiddleware {
        id: String,
    },
    ErrorObserver {
        id: String,
    },
    ErrorHandler {
        error_ref_input_index: usize,
        default: Option<bool>,
        id: String,
    },
    Route {
        method: MethodGuard,
        path: String,
    },
    Fallback {
        id: String,
    },
    Methods,
}

impl AnnotationProperties {
    fn from_meta(kind: AnnotationKind, item: &syn::Meta) -> Result<Self, InvalidAttributeParams> {
        use AnnotationKind::*;
        use model::*;

        match kind {
            Constructor => ConstructorProperties::from_meta(item).map(Into::into),
            Config => ConfigProperties::from_meta(item).map(Into::into),
            WrappingMiddleware => WrappingMiddlewareProperties::from_meta(item).map(Into::into),
            PreProcessingMiddleware => {
                PreProcessingMiddlewareProperties::from_meta(item).map(Into::into)
            }
            PostProcessingMiddleware => {
                PostProcessingMiddlewareProperties::from_meta(item).map(Into::into)
            }
            ErrorObserver => ErrorObserverProperties::from_meta(item).map(Into::into),
            ErrorHandler => ErrorHandlerProperties::from_meta(item).map(Into::into),
            Prebuilt => PrebuiltProperties::from_meta(item).map(Into::into),
            Route => RouteProperties::from_meta(item).map(Into::into),
            Fallback => FallbackProperties::from_meta(item).map(Into::into),
            Methods => Ok(AnnotationProperties::Methods),
        }
        .map_err(|e| InvalidAttributeParams::new(e, kind))
    }

    /// Return the id of this component, if one was set.
    pub fn id(&self) -> Option<&str> {
        use AnnotationProperties::*;

        match self {
            WrappingMiddleware { id }
            | PreProcessingMiddleware { id }
            | PostProcessingMiddleware { id }
            | ErrorObserver { id }
            | ErrorHandler { id, .. }
            | Fallback { id } => Some(id.as_str()),
            Constructor { .. } | Prebuilt { .. } | Config { .. } | Route { .. } | Methods => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationKind {
    Constructor,
    Config,
    WrappingMiddleware,
    PreProcessingMiddleware,
    PostProcessingMiddleware,
    ErrorObserver,
    ErrorHandler,
    Prebuilt,
    Route,
    Fallback,
    Methods,
}

impl std::fmt::Display for AnnotationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnnotationKind::Constructor => write!(f, "constructor"),
            AnnotationKind::Config => write!(f, "config"),
            AnnotationKind::WrappingMiddleware => write!(f, "wrap"),
            AnnotationKind::PreProcessingMiddleware => write!(f, "pre_process"),
            AnnotationKind::PostProcessingMiddleware => write!(f, "post_process"),
            AnnotationKind::ErrorObserver => write!(f, "error_observer"),
            AnnotationKind::ErrorHandler => write!(f, "error_handler"),
            AnnotationKind::Prebuilt => write!(f, "prebuilt"),
            AnnotationKind::Route => write!(f, "route"),
            AnnotationKind::Fallback => write!(f, "fallback"),
            AnnotationKind::Methods => write!(f, "methods"),
        }
    }
}

impl AnnotationKind {
    fn parse(ident: &syn::Ident) -> Result<AnnotationKind, ()> {
        match ident.to_string().as_str() {
            "constructor" => Ok(AnnotationKind::Constructor),
            "config" => Ok(AnnotationKind::Config),
            "wrap" => Ok(AnnotationKind::WrappingMiddleware),
            "post_process" => Ok(AnnotationKind::PostProcessingMiddleware),
            "pre_process" => Ok(AnnotationKind::PreProcessingMiddleware),
            "error_observer" => Ok(AnnotationKind::ErrorObserver),
            "prebuilt" => Ok(AnnotationKind::Prebuilt),
            "route" => Ok(AnnotationKind::Route),
            "fallback" => Ok(AnnotationKind::Fallback),
            "error_handler" => Ok(AnnotationKind::ErrorHandler),
            "methods" => Ok(AnnotationKind::Methods),
            _ => Err(()),
        }
    }

    pub fn diagnostic_attribute(&self) -> &'static str {
        use AnnotationKind::*;

        match self {
            Constructor => "pavex::diagnostic::constructor",
            Config => "pavex::diagnostic::config",
            WrappingMiddleware => "pavex::diagnostic::wrap",
            PreProcessingMiddleware => "pavex::diagnostic::pre_process",
            PostProcessingMiddleware => "pavex::diagnostic::post_process",
            ErrorObserver => "pavex::diagnostic::error_observer",
            ErrorHandler => "pavex::diagnostic::error_handler",
            Prebuilt => "pavex::diagnostic::prebuilt",
            Route => "pavex::diagnostic::route",
            Fallback => "pavex::diagnostic::fallback",
            Methods => "pavex::diagnostic::methods",
        }
    }
}

impl AnnotationProperties {
    pub fn attribute(&self) -> &'static str {
        self.kind().diagnostic_attribute()
    }

    pub fn kind(&self) -> AnnotationKind {
        match self {
            AnnotationProperties::Constructor { .. } => AnnotationKind::Constructor,
            AnnotationProperties::Config { .. } => AnnotationKind::Config,
            AnnotationProperties::WrappingMiddleware { .. } => AnnotationKind::WrappingMiddleware,
            AnnotationProperties::PreProcessingMiddleware { .. } => {
                AnnotationKind::PreProcessingMiddleware
            }
            AnnotationProperties::PostProcessingMiddleware { .. } => {
                AnnotationKind::PostProcessingMiddleware
            }
            AnnotationProperties::ErrorObserver { .. } => AnnotationKind::ErrorObserver,
            AnnotationProperties::Prebuilt { .. } => AnnotationKind::Prebuilt,
            AnnotationProperties::Route { .. } => AnnotationKind::Route,
            AnnotationProperties::Fallback { .. } => AnnotationKind::Fallback,
            AnnotationProperties::ErrorHandler { .. } => AnnotationKind::ErrorHandler,
            AnnotationProperties::Methods => AnnotationKind::Methods,
        }
    }
}

/// Strip the `diagnostic::pavex` prefix from a path.
///
/// It returns `None` if the path doesn't start with `diagnostic::pavex`.
/// It returns `Some` otherwise, yielding the remaining path segments.
fn strip_pavex_path_prefix(path: &syn::Path) -> Option<syn::Path> {
    if path.segments.len() < 2 {
        return None;
    }

    let prefix = &path.segments[0];
    let pavex = &path.segments[1];

    if prefix.ident == "diagnostic"
        && prefix.arguments.is_empty()
        && pavex.ident == "pavex"
        && pavex.arguments.is_empty()
    {
        let remaining_segments = path.segments.iter().skip(2).cloned().collect();
        let remaining_path = syn::Path {
            leading_colon: path.leading_colon,
            segments: remaining_segments,
        };
        Some(remaining_path)
    } else {
        None
    }
}

/// Extract outer attributes from a string of attributes.
/// I.e. attributes that start with `#[` rather than `#![`.
fn parse_outer_attrs(attrs: &str) -> syn::Result<Vec<syn::Attribute>> {
    /// `syn` doesn't let you parse outer attributes directly since `syn::Attribute` doesn't
    /// implement `syn::parse::Parse`.
    /// We use this thin wrapper to be able to invoke `syn::Attribute::parse_outer` on it.
    struct OuterAttributes(Vec<syn::Attribute>);

    impl syn::parse::Parse for OuterAttributes {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            input.call(syn::Attribute::parse_outer).map(OuterAttributes)
        }
    }
    syn::parse_str::<OuterAttributes>(attrs).map(|a| a.0)
}
