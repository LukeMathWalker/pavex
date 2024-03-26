use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

pub struct A;

pub fn a() -> A {
    todo!()
}

pub fn first(_response: Response, _a: &mut A) -> Response {
    todo!()
}

pub fn second(_response: Response, _a: &mut A) -> Response {
    todo!()
}

pub fn third(_response: Response, _a: A) -> Response {
    todo!()
}

pub fn handler(_a: &mut A) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::a));
    bp.post_process(f!(crate::first));
    bp.post_process(f!(crate::second));
    bp.post_process(f!(crate::third));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
