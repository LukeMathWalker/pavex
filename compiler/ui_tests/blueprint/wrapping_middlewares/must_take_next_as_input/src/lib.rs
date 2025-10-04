use pavex::Response;
use pavex::{blueprint::from, Blueprint};

#[pavex::wrap]
pub fn mw() -> Response {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(MW);
    bp.routes(from![crate]);
    bp
}
