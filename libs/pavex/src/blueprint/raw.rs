use super::reflection::AnnotationCoordinates;

macro_rules! raw {
    ($name:ident) => {
        pub struct $name {
            pub coordinates: AnnotationCoordinates,
        }
    };
}

raw!(RawPreProcessingMiddleware);
raw!(RawPostProcessingMiddleware);
raw!(RawWrappingMiddleware);
raw!(RawErrorHandler);
raw!(RawErrorObserver);
raw!(RawFallback);
