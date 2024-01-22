use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::request::path::RawPathParams;
use pavex::response::Response;

pub fn mw<T>(_next: Next<T>) -> Response
where
    T: std::future::IntoFuture<Output = Response>,
{
    todo!()
}

pub fn handler(_s: &RawPathParams) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(f!(crate::mw));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
