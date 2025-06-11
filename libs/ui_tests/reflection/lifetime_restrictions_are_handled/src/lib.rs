use pavex::blueprint::{from, Blueprint};
use pavex::middleware::Next;
use pavex::request::path::RawPathParams;
use pavex::response::Response;

#[pavex::wrap]
pub fn mw<T>(_next: Next<T>) -> Response
where
    T: std::future::IntoFuture<Output = Response>,
{
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_s: &RawPathParams) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![pavex, crate]);
    bp.wrap(MW);
    bp.routes(from![crate]);
    bp
}
