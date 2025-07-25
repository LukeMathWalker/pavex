use pavex::Response;
use pavex::{blueprint::from, Blueprint};

#[pavex::route(path = "/", allow(any_method))]
pub fn any_root() -> Response {
    todo!()
}

#[pavex::get(path = "/")]
pub fn get_root() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate]);
    bp
}
