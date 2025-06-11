use pavex::blueprint::{from, Blueprint};
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

#[pavex::methods]
impl Singleton {
    #[singleton]
    pub fn new() -> Singleton {
        todo!()
    }
}

pub struct RequestScoped;

#[pavex::request_scoped]
pub fn request_scoped(_s: &Singleton) -> RequestScoped {
    todo!()
}

#[pavex::wrap]
pub fn wrap<T: IntoFuture<Output = Response>>(_next: Next<T>) -> Response {
    todo!()
}

#[pavex::post_process]
pub fn post(_r: Response, _x: &RequestScoped) -> Response {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_r: &RequestScoped) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate, pavex]);
    bp.wrap(WRAP);
    bp.post_process(POST);
    bp.routes(from![crate]);
    bp
}
