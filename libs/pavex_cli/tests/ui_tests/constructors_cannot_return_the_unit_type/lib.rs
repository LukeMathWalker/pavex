use pavex_builder::{f, AppBlueprint, Lifecycle};

pub fn bogus_constructor() {
    todo!()
}

pub fn handler() -> http::Response<hyper::body::Body> {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new()
        .constructor(f!(crate::bogus_constructor), Lifecycle::Singleton)
        .route(f!(crate::handler), "/home")
}
