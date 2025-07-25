use pavex::middleware::Next;
use pavex::Response;
use pavex::{blueprint::from, Blueprint};

#[derive(Clone)]
pub struct A;

#[pavex::request_scoped(never_clone, id = "A_")]
pub fn a() -> A {
    todo!()
}

#[pavex::wrap]
pub fn wrap<T>(_next: Next<T>, _a: A) -> Response
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

#[pavex::post_process]
pub fn post(_response: Response, _a: &A) -> Response {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.post_process(POST);
    bp.wrap(WRAP);
    bp.routes(from![crate]);
    bp
}
