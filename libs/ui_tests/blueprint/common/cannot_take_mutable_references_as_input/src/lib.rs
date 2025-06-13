use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::f;
use pavex::methods;
use pavex::middleware::Next;
use pavex::request::RequestHead;
use pavex::response::Response;

pub struct B;

pub fn constructor(_r: &mut RequestHead) -> B {
    todo!()
}

pub struct A;

#[methods]
impl A {
    #[pavex::request_scoped]
    pub fn new(_r: &mut RequestHead) -> Self {
        todo!()
    }
}

pub fn error_handler(_e: &pavex::Error, _s: &mut B) -> Response {
    todo!()
}

#[pavex::wrap]
pub fn wrapping<C>(_next: Next<C>, _s: &mut B) -> Response
where
    C: std::future::IntoFuture<Output = Response>,
{
    todo!()
}

#[pavex::error_observer]
pub fn observer(_e: &pavex::Error, _s: &mut A) {
    todo!()
}

pub fn handler(_s: &A) -> Result<Response, pavex::Error> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.request_scoped(f!(crate::constructor));
    bp.wrap(WRAPPING);
    bp.error_observer(OBSERVER);
    bp.route(GET, "/home", f!(crate::handler))
        .error_handler(f!(crate::error_handler));
    bp
}
