use super::reflection::{RawIdentifiers, WithLocation};

pub struct RawPreProcessingMiddleware {
    pub coordinates: WithLocation<RawIdentifiers>,
    pub error_handler: Option<RawErrorHandler>,
}

pub struct RawPostProcessingMiddleware {
    pub coordinates: WithLocation<RawIdentifiers>,
    pub error_handler: Option<RawErrorHandler>,
}

pub struct RawWrappingMiddleware {
    pub coordinates: WithLocation<RawIdentifiers>,
    pub error_handler: Option<RawErrorHandler>,
}

pub struct RawErrorHandler {
    pub coordinates: WithLocation<RawIdentifiers>,
}
