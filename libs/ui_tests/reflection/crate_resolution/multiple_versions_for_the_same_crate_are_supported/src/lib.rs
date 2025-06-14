use pavex::blueprint::{from, Blueprint};
use pavex::f;

pub fn header1() -> http_01::header::HeaderName {
    todo!()
}

pub fn header2() -> http_02::header::HeaderName {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(
    _h1: http_01::header::HeaderName,
    _h2: http_02::header::HeaderName,
) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f![crate::header1]);
    bp.request_scoped(f![crate::header2]);
    bp.routes(from![crate]);
    bp
}
