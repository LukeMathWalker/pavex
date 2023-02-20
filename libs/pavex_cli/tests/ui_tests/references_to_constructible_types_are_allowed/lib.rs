use pavex_builder::{f, Blueprint, Lifecycle};

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
) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::Singleton::new), Lifecycle::Singleton);
    bp.constructor(f!(crate::request_scoped), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::transient), Lifecycle::Transient);
    bp.route(f!(crate::stream_file), "/home");
    bp
}
