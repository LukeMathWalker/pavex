use pavex::Response;
use pavex::{blueprint::from, Blueprint};

#[pavex::post_process]
pub fn mw() -> Response {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.post_process(MW);
    bp.routes(from![crate]);
    bp
}
