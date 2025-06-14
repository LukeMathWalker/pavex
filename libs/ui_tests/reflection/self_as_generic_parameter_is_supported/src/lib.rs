use pavex::blueprint::{from, Blueprint};
use pavex::f;

pub struct A {}

impl A {
    pub fn new() -> anyhow::Result<Self> {
        todo!()
    }
}

pub fn error_handler(_err: &anyhow::Error) -> pavex::response::Response {
    todo!()
}

pub fn handler(_inner: A) -> pavex::response::Response {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn route_handler(_inner: A) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::A::new))
        .error_handler(f!(crate::error_handler));
    bp.routes(from![crate]);
    bp
}
