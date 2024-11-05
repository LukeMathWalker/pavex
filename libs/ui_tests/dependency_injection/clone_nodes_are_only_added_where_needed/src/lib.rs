use std::future::IntoFuture;

use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;

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

pub fn mw<C>(_s: Singleton, _next: Next<C>) -> Response
where
    C: IntoFuture<Output = Response>,
{
    todo!()
}

pub fn handler(_s: Singleton) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(crate::Singleton::new)).clone_if_necessary();
    bp.wrap(f!(crate::mw));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
