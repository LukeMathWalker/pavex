use pavex_builder::{f, AppBlueprint, Lifecycle};

#[derive(Clone)]
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

pub struct Transient;

pub fn transient() -> Transient {
    todo!()
}

pub fn stream_file(
    s: &Singleton,
    r: &RequestScoped,
    t: &Transient,
) -> http::Response<hyper::body::Body> {
    todo!()
}

pub fn blueprint() -> AppBlueprint {
    AppBlueprint::new()
        .constructor(f!(crate::Singleton::new), Lifecycle::Singleton)
        .constructor(f!(crate::request_scoped), Lifecycle::RequestScoped)
        .constructor(f!(crate::transient), Lifecycle::Transient)
        .route(f!(crate::stream_file), "/home")
}
