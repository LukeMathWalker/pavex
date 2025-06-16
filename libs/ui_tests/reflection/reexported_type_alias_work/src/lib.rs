use pavex::blueprint::{from, Blueprint};
use pavex::response::Response;

pub type RemoteAlias = dep::Surreal<dep::engine::Any>;

#[pavex::singleton]
pub fn constructor() -> RemoteAlias {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_a: &RemoteAlias) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
