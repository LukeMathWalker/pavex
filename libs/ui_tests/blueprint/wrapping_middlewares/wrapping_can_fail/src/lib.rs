use pavex::blueprint::{from, Blueprint};
use pavex::middleware::Next;
use pavex::response::Response;

#[pavex::wrap(
    id = "EHANDLER_VIA_ATTRIBUTE",
    error_handler = "crate::CustomError::into_response"
)]
pub fn via_attribute<T>(_next: Next<T>) -> Result<Response, CustomError>
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

#[pavex::wrap(id = "EHANDLER_VIA_BLUEPRINT")]
// Error handler isn't specified at the macro-level, it's added
// directly when the middleware is registered against the blueprint.
pub fn via_blueprint<T>(_next: Next<T>) -> Result<Response, CustomError>
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

#[pavex::wrap(
    id = "EHANDLER_OVERRIDE_VIA_BLUEPRINT",
    error_handler = "crate::CustomError::into_response"
)]
// Error handler is specified at the macro-level, but it can be
// overridden when the middleware is registered against the blueprint.
pub fn override_in_blueprint<T>(_next: Next<T>) -> Result<Response, CustomError>
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler() -> Response {
    todo!()
}

#[derive(Debug)]
pub struct CustomError;

#[pavex::methods]
impl CustomError {
    #[error_handler(default = false)]
    pub fn into_response(&self) -> Response {
        todo!()
    }

    #[error_handler(default = false)]
    pub fn into_response_override(&self) -> Response {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.wrap(EHANDLER_VIA_ATTRIBUTE);
    bp.wrap(EHANDLER_VIA_BLUEPRINT)
        .error_handler(CUSTOM_ERROR_INTO_RESPONSE);
    bp.wrap(EHANDLER_OVERRIDE_VIA_BLUEPRINT)
        .error_handler(CUSTOM_ERROR_INTO_RESPONSE_OVERRIDE);

    bp.routes(from![crate]);
    bp
}
