use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;
use std::future::IntoFuture;

pub fn request_scoped() -> Result<String, ErrorType> {
    todo!()
}

#[derive(Debug)]
pub struct ErrorType;

// It doesn't implement IntoResponse!
pub struct MyCustomOutputType;

pub fn handler(_s: String) -> Result<MyCustomOutputType, ErrorType> {
    todo!()
}

pub fn error_handler(e: &ErrorType) -> MyCustomOutputType {
    todo!()
}

pub fn wrapping_middleware<T>(_next: Next<T>) -> Result<MyCustomOutputType, ErrorType>
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

pub fn pp_middleware<T>(_response: Response) -> Result<MyCustomOutputType, ErrorType> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(f!(crate::wrapping_middleware))
        .error_handler(f!(crate::error_handler));
    bp.post_process(f!(crate::pp_middleware))
        .error_handler(f!(crate::error_handler));
    bp.request_scoped(f!(crate::request_scoped))
        .error_handler(f!(crate::error_handler));
    bp.route(GET, "/home", f!(crate::handler))
        .error_handler(f!(crate::error_handler));
    bp
}
