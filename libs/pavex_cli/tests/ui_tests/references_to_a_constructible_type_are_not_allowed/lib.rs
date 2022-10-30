use pavex_builder::{f, AppBlueprint, Lifecycle};

pub struct Singleton;

impl Singleton {
    pub fn new() -> Singleton {
        todo!()
    }
}

pub struct RequestScoped;

pub fn request_scoped() -> RequestScoped {
    todo!()
}

pub fn stream_file(s: &Singleton, r: &RequestScoped) -> http::Response<hyper::body::Body> {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new()
        .constructor(f!(crate::Singleton::new), Lifecycle::Singleton)
        .constructor(f!(crate::request_scoped), Lifecycle::RequestScoped)
        .route(f!(crate::stream_file), "/home")
}
