use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;
use std::future::IntoFuture;

#[derive(Clone)]
pub struct Singleton;

impl Singleton {
    pub fn new() -> Singleton {
        todo!()
    }
}

pub struct RequestScoped;

pub fn request_scoped(s: &Singleton) -> RequestScoped {
    todo!()
}

pub fn wrap<T: IntoFuture<Output = Response>>(next: Next<T>) -> Response {
    todo!()
}

pub fn post(_r: Response, _x: &RequestScoped) -> Response {
    todo!()
}

pub fn handler(r: &RequestScoped) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::Singleton::new), Lifecycle::Singleton);
    bp.constructor(f!(crate::request_scoped), Lifecycle::RequestScoped);
    bp.wrap(f!(crate::wrap));
    bp.post_process(f!(crate::post));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
