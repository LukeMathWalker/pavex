use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::request::RequestHead;
use pavex::response::Response;

pub fn constructor(r: &mut RequestHead) -> String {
    todo!()
}

pub fn error_handler(e: &pavex::Error, s: &mut String) -> Response {
    todo!()
}

pub fn wrapping<C>(next: Next<C>, s: &mut String) -> Response
where
    C: std::future::IntoFuture<Output = Response>,
{
    todo!()
}

pub fn observer(e: &pavex::Error, s: &mut String) {
    todo!()
}

pub fn handler(s: &String) -> Result<Response, pavex::Error> {
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
