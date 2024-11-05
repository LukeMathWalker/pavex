use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;
use std::future::IntoFuture;

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

pub fn request_scoped(_s: &Singleton) -> RequestScoped {
    todo!()
}

pub fn wrap<T: IntoFuture<Output = Response>>(_next: Next<T>) -> Response {
    todo!()
}

pub fn post(_r: Response, _x: &RequestScoped) -> Response {
    todo!()
}

pub fn handler(_r: &RequestScoped) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(crate::Singleton::new));
    bp.request_scoped(f!(crate::request_scoped));
    bp.wrap(f!(crate::wrap));
    bp.post_process(f!(crate::post));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
