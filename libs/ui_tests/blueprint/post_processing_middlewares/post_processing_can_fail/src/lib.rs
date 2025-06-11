use pavex::blueprint::{from, Blueprint};
use pavex::f;
use pavex::response::Response;

#[pavex::post_process(
    id = "EHANDLER_VIA_ATTRIBUTE",
    error_handler = "crate::CustomError::into_response"
)]
pub fn via_attribute(_response: Response) -> Result<Response, CustomError> {
    todo!()
}

#[pavex::post_process(id = "EHANDLER_VIA_BLUEPRINT")]
// Error handler isn't specified at the macro-level, it's added
// directly when the middleware is registered against the blueprint.
pub fn via_blueprint(_response: Response) -> Result<Response, CustomError> {
    todo!()
}

#[pavex::post_process(
    id = "EHANDLER_OVERRIDE_VIA_BLUEPRINT",
    error_handler = "crate::CustomError::into_response"
)]
// Error handler is specified at the macro-level, but it can be
// overridden when the middleware is registered against the blueprint.
pub fn override_in_blueprint(_response: Response) -> Result<Response, CustomError> {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler() -> Response {
    todo!()
}

#[derive(Debug)]
pub struct CustomError;

impl CustomError {
    pub fn into_response(&self) -> Response {
        todo!()
    }

    pub fn into_response_override(&self) -> Response {
        todo!()
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.post_process(EHANDLER_VIA_ATTRIBUTE);
    bp.post_process(EHANDLER_VIA_BLUEPRINT)
        .error_handler(f!(crate::CustomError::into_response));
    bp.post_process(EHANDLER_OVERRIDE_VIA_BLUEPRINT)
        .error_handler(f!(crate::CustomError::into_response_override));

    bp.routes(from![crate]);
    bp
}
