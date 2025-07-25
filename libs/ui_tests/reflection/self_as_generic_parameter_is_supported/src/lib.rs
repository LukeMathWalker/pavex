use pavex::{blueprint::from, Blueprint};

pub struct A {}

#[pavex::methods]
impl A {
    #[request_scoped]
    pub fn new() -> anyhow::Result<Self> {
        todo!()
    }
}

#[pavex::error_handler]
pub fn error_handler(_err: &anyhow::Error) -> pavex::Response {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_inner: A) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
