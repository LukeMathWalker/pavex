use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::request::RequestHead;
use pavex::response::Response;

pub fn constructor(_r: &mut RequestHead) -> String {
    todo!()
}

pub fn error_handler(_e: &pavex::Error, _s: &mut String) -> Response {
    todo!()
}

pub fn wrapping<C>(_next: Next<C>, _s: &mut String) -> Response
where
    C: std::future::IntoFuture<Output = Response>,
{
    todo!()
}

pub fn observer(_e: &pavex::Error, _s: &mut String) {
    todo!()
}

pub fn handler(_s: &String) -> Result<Response, pavex::Error> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::constructor));
    bp.wrap(f!(crate::wrapping));
    bp.error_observer(f!(crate::observer));
    bp.route(GET, "/home", f!(crate::handler))
        .error_handler(f!(crate::error_handler));
    bp
}
