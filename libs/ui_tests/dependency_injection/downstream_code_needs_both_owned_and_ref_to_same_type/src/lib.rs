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
pub struct Scoped;

impl Scoped {
    pub fn new() -> Scoped {
        todo!()
    }
}

pub fn mw<C>(s: Scoped, next: Next<C>) -> Response
where
    C: IntoFuture<Output = Response>,
{
    todo!()
}

pub fn mw2<C>(s: &Scoped, next: Next<C>) -> Response
where
    C: IntoFuture<Output = Response>,
{
    todo!()
}

pub fn handler(s: Scoped) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::Scoped::new), Lifecycle::RequestScoped)
        .clone_if_necessary();
    bp.wrap(f!(crate::mw));
    bp.wrap(f!(crate::mw2));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
