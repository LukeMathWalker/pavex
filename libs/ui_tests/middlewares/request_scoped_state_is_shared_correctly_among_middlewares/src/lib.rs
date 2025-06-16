use pavex::blueprint::{from, Blueprint};
use pavex::middleware::{Next, Processing};
use pavex::response::Response;

pub struct A;

#[pavex::request_scoped(id = "A_")]
pub fn a() -> A {
    todo!()
}

#[pavex::pre_process]
pub fn pre(_a: &A) -> Processing {
    todo!()
}

#[pavex::wrap]
pub fn wrap<T: IntoFuture<Output = Response>>(_next: Next<T>, _a: &A) -> Response {
    todo!()
}

#[pavex::post_process]
pub fn post(_response: Response, _a: A) -> Response {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.pre_process(PRE);
    bp.post_process(POST);
    bp.wrap(WRAP);
    bp.routes(from![crate]);
    bp
}
