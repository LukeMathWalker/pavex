use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

pub type RemoteAlias = dep::Surreal<dep::engine::Any>;

pub fn constructor() -> RemoteAlias {
    todo!()
}

pub fn handler(_a: &RemoteAlias) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(crate::constructor));
    bp.route(GET, "/", f!(crate::handler));
    bp
}
