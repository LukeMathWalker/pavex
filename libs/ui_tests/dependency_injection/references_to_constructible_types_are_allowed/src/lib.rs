use pavex::blueprint::{router::GET, Blueprint};
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
    bp.route(GET, "/home", f!(crate::stream_file));
    bp
}
