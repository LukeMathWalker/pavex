use std::future::IntoFuture;

use pavex::blueprint::{
    constructor::{CloningStrategy, Lifecycle},
    router::GET,
    Blueprint,
};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;

#[derive(Clone)]
pub struct Singleton;

impl Singleton {
    pub fn new() -> Singleton {
        todo!()
    }
}

pub fn mw<C>(s: Singleton, next: Next<C>) -> Response
where
    C: IntoFuture<Output = Response>,
{
    todo!()
}

pub fn handler(s: Singleton) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::Singleton::new), Lifecycle::Singleton)
        .clone_if_necessary();
    bp.wrap(f!(crate::mw));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
