use pavex::response::Response;
use pavex::{blueprint::from, Blueprint};

pub struct A;

#[pavex::request_scoped(id = "A_")]
pub fn a() -> A {
    todo!()
}

#[pavex::post_process]
pub fn first(_response: Response, _a: &mut A) -> Response {
    todo!()
}

#[pavex::post_process]
pub fn second(_response: Response, _a: &mut A) -> Response {
    todo!()
}

#[pavex::post_process]
pub fn third(_response: Response, _a: A) -> Response {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_a: &mut A) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.post_process(FIRST);
    bp.post_process(SECOND);
    bp.post_process(THIRD);
    bp.routes(from![crate]);
    bp
}
