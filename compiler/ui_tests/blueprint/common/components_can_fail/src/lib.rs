use pavex::Response;
use pavex::{blueprint::from, Blueprint};

pub struct B;

#[pavex::request_scoped(id = "B_")]
pub fn b() -> Result<B, ErrorB> {
    todo!()
}

#[derive(Debug)]
pub struct ErrorB;

#[pavex::methods]
impl ErrorB {
    #[pavex::error_handler]
    pub fn into_response(&self) -> pavex::Response {
        todo!()
    }
}

pub struct Singleton;

// No need to specify an error handler for singleton constructors.
// The error is bubbled up by `ApplicationState::new`.
#[pavex::singleton]
pub fn singleton() -> Result<Singleton, SingletonError> {
    todo!()
}

#[derive(Debug, thiserror::Error)]
#[error("The error message")]
pub struct SingletonError;

#[pavex::get(path = "/")]
pub fn handler(_b: &B, _singleton: &Singleton) -> Result<Response, CustomError> {
    todo!()
}

#[derive(Debug)]
pub struct CustomError;

#[pavex::methods]
impl CustomError {
    #[pavex::error_handler]
    pub fn into_response(&self) -> Response {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from!(crate));
    bp.routes(from![crate]);
    bp
}
