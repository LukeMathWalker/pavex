use pavex::Response;
use pavex::Blueprint;

#[derive(Debug)]
pub struct CustomError;

pub struct Generic<T> {
    _t: T,
}

impl std::fmt::Display for CustomError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for CustomError {}

#[pavex::request_scoped]
pub fn fallible_constructor() -> Result<String, CustomError> {
    todo!()
}

#[pavex::request_scoped]
pub fn generic_fallible_constructor<T>() -> Result<Generic<T>, CustomError> {
    todo!()
}

#[pavex::error_handler(default = false)]
pub fn error_handler(_e: &pavex::Error) -> Response {
    todo!()
}

#[pavex::error_observer]
pub fn error_observer(_e: &pavex::Error) {
    todo!()
}

#[pavex::get(path = "/without_observer")]
pub fn without_observer(_s: String, _t: Generic<String>) -> Response {
    todo!()
}

#[pavex::get(path = "/with_observer")]
pub fn with_observer(_s: String, _t: Generic<String>) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(FALLIBLE_CONSTRUCTOR)
        .error_handler(ERROR_HANDLER);
    bp.constructor(GENERIC_FALLIBLE_CONSTRUCTOR)
        .error_handler(ERROR_HANDLER);

    // We test the behaviour with and without error observers.
    bp.route(WITHOUT_OBSERVER);

    bp.nest({
        let mut bp = Blueprint::new();
        bp.error_observer(ERROR_OBSERVER);
        bp.route(WITH_OBSERVER);
        bp
    });
    bp
}
