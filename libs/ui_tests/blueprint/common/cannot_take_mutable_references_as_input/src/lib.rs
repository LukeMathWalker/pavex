use pavex::blueprint::{from, Blueprint};
use pavex::methods;
use pavex::middleware::Next;
use pavex::request::RequestHead;
use pavex::response::Response;

pub struct A;

#[methods]
impl A {
    #[pavex::request_scoped]
    pub fn new(_r: &mut RequestHead) -> Self {
        todo!()
    }
}

#[pavex::error_handler]
pub fn error_handler(#[px(error_ref)] _e: &pavex::Error, _s: &mut A) -> Response {
    todo!()
}

#[pavex::wrap]
pub fn wrapping<C>(_next: Next<C>, _s: &mut A) -> Response
where
    C: std::future::IntoFuture<Output = Response>,
{
    todo!()
}

#[pavex::error_observer]
pub fn observer(_e: &pavex::Error, _s: &mut A) {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_s: &A) -> Result<Response, pavex::Error> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.wrap(WRAPPING);
    bp.error_observer(OBSERVER);
    bp.routes(from![crate]);
    bp
}
