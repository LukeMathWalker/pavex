use super::reflection::{RawIdentifiers, WithLocation};

pub struct RawPreProcessingMiddleware {
    pub coordinates: WithLocation<RawIdentifiers>,
}

pub struct RawPostProcessingMiddleware {
    pub coordinates: WithLocation<RawIdentifiers>,
}

pub struct RawWrappingMiddleware {
    pub coordinates: WithLocation<RawIdentifiers>,
}

pub struct RawErrorHandler {
    pub coordinates: WithLocation<RawIdentifiers>,
}
