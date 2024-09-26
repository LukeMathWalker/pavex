use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

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

pub fn constructor<S>() -> Result<Generic<S>, GenericError<S>> {
    todo!()
}

pub fn handler(_b: Generic<String>) -> Response {
    todo!()
}

pub fn error_handler<S>(_e: &GenericError<S>) -> Response {
    todo!()
}

pub fn error_observer(_err: &pavex::Error) {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::constructor), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));
    bp.error_observer(f!(crate::error_observer));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
