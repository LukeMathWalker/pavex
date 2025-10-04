use pavex::blueprint::from;
use pavex::Blueprint;

pub struct A;

#[pavex::singleton]
pub fn conflict() -> A {
    todo!()
}

pub mod routes {
    use pavex::Response;

    #[pavex::get(path = "/", id = "CONFLICT")]
    pub fn handler() -> Response {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
