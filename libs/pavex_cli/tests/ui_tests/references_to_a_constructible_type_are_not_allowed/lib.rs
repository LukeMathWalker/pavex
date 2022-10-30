use pavex_builder::{f, AppBlueprint, Lifecycle};

pub struct Logger;

impl Logger {
    pub fn new() -> Logger {
        todo!()
    }
}

pub fn stream_file(input: &Logger) -> http::Response<hyper::body::Body> {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new()
        .constructor(f!(crate::Logger::new), Lifecycle::Singleton)
        .route(f!(crate::stream_file), "/home")
}
