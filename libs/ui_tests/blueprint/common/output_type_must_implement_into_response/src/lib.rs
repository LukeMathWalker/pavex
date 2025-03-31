use pavex::blueprint::{from, router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;
use std::future::IntoFuture;

pub struct A;
pub struct B;

pub fn a() -> Result<A, ErrorType> {
    todo!()
}

#[pavex::transient(error_handler = "crate::error_handler")]
pub fn b() -> Result<B, ErrorType> {
    todo!()
}

#[derive(Debug)]
pub struct ErrorType;

// It doesn't implement IntoResponse!
pub struct BrokenOutput;

pub fn handler(_a: &A, _b: B) -> Result<BrokenOutput, ErrorType> {
    todo!()
}

pub fn error_handler(_e: &ErrorType) -> BrokenOutput {
    todo!()
}

pub fn wrap<T>(_next: Next<T>) -> Result<BrokenOutput, ErrorType>
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

pub fn pp<T>(_response: Response) -> Result<BrokenOutput, ErrorType> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.request_scoped(f!(crate::a))
        .error_handler(f!(crate::error_handler));
    bp.wrap(f!(crate::wrap))
        .error_handler(f!(crate::error_handler));
    bp.post_process(f!(crate::pp))
        .error_handler(f!(crate::error_handler));
    bp.route(GET, "/", f!(crate::handler))
        .error_handler(f!(crate::error_handler));
    bp
}
