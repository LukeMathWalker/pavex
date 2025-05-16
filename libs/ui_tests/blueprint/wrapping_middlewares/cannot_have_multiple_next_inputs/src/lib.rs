use std::future::IntoFuture;

use pavex::blueprint::{router::GET, Blueprint};
use pavex::middleware::Next;
use pavex::response::Response;
use pavex::{f, wrap};

pub fn mw<T>(_next: Next<T>, _second_next: Next<T>) -> Response
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

#[wrap]
pub fn mw1<T>(_next: Next<T>, _second_next: Next<T>) -> Response
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(f!(crate::mw));
    bp.wrap(MW_1);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
