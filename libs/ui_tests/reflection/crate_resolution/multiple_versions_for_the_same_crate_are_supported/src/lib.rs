use pavex::{blueprint::from, Blueprint};

#[pavex::request_scoped]
pub fn header1() -> http_01::header::HeaderName {
    todo!()
}

#[pavex::request_scoped]
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
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
