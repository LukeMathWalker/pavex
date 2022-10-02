use pavex_builder::{f, AppBlueprint, Lifecycle};

pub struct Logger<T>(T);

pub fn new_logger<T>() -> Logger<T> {
    todo!()
}

pub fn handler<T>(logger: Logger<T>) -> http::Response<hyper::body::Body> {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new()
        .constructor(f!(crate::new_logger::<String>), Lifecycle::Singleton)
        .route(f!(crate::handler::<std::string::String>), "/home")
}
