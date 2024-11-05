use std::future::IntoFuture;

use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;

#[derive(Clone)]
pub struct Scoped;

impl Default for Scoped {
    fn default() -> Self {
        Self::new()
    }
}

impl Scoped {
    pub fn new() -> Scoped {
        todo!()
    }
}

pub fn mw<C>(_s: Scoped, _next: Next<C>) -> Response
where
    C: IntoFuture<Output = Response>,
{
    todo!()
}

pub fn mw2<C>(_s: &Scoped, _next: Next<C>) -> Response
where
    C: IntoFuture<Output = Response>,
{
    todo!()
}

pub fn handler(_s: Scoped) -> pavex::response::Response {
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
