use dep::{new_logger, Logger};
use pavex_builder::{f, AppBlueprint, Lifecycle};

pub fn handler(logger: Logger) -> http::Response<hyper::body::Body> {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new()
        .constructor(f!(new_logger), Lifecycle::Singleton)
        .route(f!(crate::handler), "/home")
}
