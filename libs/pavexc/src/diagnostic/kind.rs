use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ComponentKind {
    RequestHandler,
    Constructor,
    ErrorHandler,
    WrappingMiddleware,
    PostProcessingMiddleware,
    PreProcessingMiddleware,
    ErrorObserver,
    PrebuiltType,
    ConfigType,
}

impl Display for ComponentKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ComponentKind::RequestHandler => "request handler",
            ComponentKind::Constructor => "constructor",
            ComponentKind::ErrorHandler => "error handler",
            ComponentKind::WrappingMiddleware => "wrapping middleware",
            ComponentKind::PostProcessingMiddleware => "post-processing middleware",
            ComponentKind::PreProcessingMiddleware => "pre-processing middleware",
            ComponentKind::ErrorObserver => "error observer",
            ComponentKind::PrebuiltType => "prebuilt type",
            ComponentKind::ConfigType => "config type",
        };
        write!(f, "{s}")
    }
}
