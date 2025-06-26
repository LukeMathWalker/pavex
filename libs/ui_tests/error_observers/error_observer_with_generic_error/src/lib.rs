use pavex::response::Response;
use pavex::{blueprint::from, Blueprint};

pub struct Generic<S>(S);

pub struct GenericError<S>(S);

impl<S> std::fmt::Debug for GenericError<S> {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<S> std::fmt::Display for GenericError<S> {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<S> std::error::Error for GenericError<S> {}

#[pavex::request_scoped]
pub fn constructor<S>() -> Result<Generic<S>, GenericError<S>> {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_b: Generic<String>) -> Response {
    todo!()
}

#[pavex::error_handler]
pub fn error_handler<S>(_e: &GenericError<S>) -> Response {
    todo!()
}

#[pavex::error_observer]
pub fn error_observer(_err: &pavex::Error) {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.error_observer(ERROR_OBSERVER);
    bp.routes(from![crate]);
    bp
}
