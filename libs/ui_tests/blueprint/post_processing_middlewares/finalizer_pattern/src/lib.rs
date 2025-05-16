use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::post_process;
use pavex::response::Response;

pub struct A;

pub fn a() -> A {
    todo!()
}

#[post_process]
pub fn first(_response: Response, _a: &mut A) -> Response {
    todo!()
}

#[post_process]
pub fn second(_response: Response, _a: &mut A) -> Response {
    todo!()
}

#[post_process]
pub fn third(_response: Response, _a: A) -> Response {
    todo!()
}

pub fn handler(_a: &mut A) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::a));
    bp.post_process(FIRST);
    bp.post_process(SECOND);
    bp.post_process(THIRD);
    bp.route(GET, "/", f!(crate::handler));
    bp
}
