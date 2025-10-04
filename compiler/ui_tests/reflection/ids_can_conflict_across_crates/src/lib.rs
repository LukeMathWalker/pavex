use dep_1::A;
use pavex::blueprint::from;
use pavex::Blueprint;
use pavex::Response;

#[pavex::get(path = "/", id = "CONFLICT")]
pub fn handler(_a: &A) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate, dep_1]);
    bp.routes(from![crate]);
    bp
}
