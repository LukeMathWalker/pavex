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
    let mut bp = AppBlueprint::new();
    bp.constructor(f!(crate::Singleton::new), Lifecycle::Singleton);
    bp.constructor(f!(crate::request_scoped), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::transient), Lifecycle::Transient);
    bp.route(f!(crate::stream_file), "/home");
    bp
}
