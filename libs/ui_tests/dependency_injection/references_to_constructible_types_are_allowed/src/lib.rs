use pavex::blueprint::{from, Blueprint};
use pavex::f;

#[derive(Clone)]
pub struct Singleton;

impl Default for Singleton {
    fn default() -> Self {
        Self::new()
    }
}

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

#[pavex::get(path = "/home")]
pub fn stream_file(
    _s: &Singleton,
    _r: &RequestScoped,
    _t: &Transient,
) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(crate::Singleton::new));
    bp.request_scoped(f!(crate::request_scoped));
    bp.transient(f!(crate::transient));
    bp.routes(from![crate]);
    bp
}
