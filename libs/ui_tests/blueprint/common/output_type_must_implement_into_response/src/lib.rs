use pavex::blueprint::{from, Blueprint};
use pavex::middleware::Next;
use pavex::response::Response;

pub struct A;
pub struct B;

#[pavex::request_scoped(id = "A_")]
pub fn a() -> Result<A, ErrorType> {
    todo!()
}

#[pavex::transient(id = "B_")]
pub fn b() -> Result<B, ErrorType> {
    todo!()
}

#[derive(Debug)]
pub struct ErrorType;

// It doesn't implement IntoResponse!
pub struct BrokenOutput;

#[pavex::get(path = "/")]
pub fn handler(_a: &A, _b: B) -> Result<BrokenOutput, ErrorType> {
    todo!()
}

#[pavex::error_handler]
pub fn error_handler(_e: &ErrorType) -> BrokenOutput {
    todo!()
}

#[pavex::wrap]
pub fn wrap<T>(_next: Next<T>) -> Result<BrokenOutput, ErrorType>
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

#[pavex::post_process]
pub fn pp<T>(_response: Response) -> Result<BrokenOutput, ErrorType> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.wrap(WRAP);
    bp.post_process(PP);
    bp.routes(from![crate]);
    bp
}
