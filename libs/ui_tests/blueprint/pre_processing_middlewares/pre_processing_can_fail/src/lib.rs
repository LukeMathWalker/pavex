use pavex::middleware::Processing;
use pavex::response::Response;
use pavex::{blueprint::from, Blueprint};

#[pavex::pre_process(id = "EHANDLER_VIA_DEFAULT")]
/// The default error handler is invoked.
pub fn via_attribute() -> Result<Processing, CustomError> {
    todo!()
}

#[pavex::pre_process(id = "EHANDLER_OVERRIDE_VIA_BLUEPRINT")]
// Error handler is overridden when the middleware is registered against the blueprint.
pub fn override_in_blueprint() -> Result<Processing, CustomError> {
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
    #[error_handler]
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
    bp.import(from![crate]);

    bp.pre_process(EHANDLER_VIA_DEFAULT);
    bp.pre_process(EHANDLER_OVERRIDE_VIA_BLUEPRINT)
        .error_handler(CUSTOM_ERROR_INTO_RESPONSE_OVERRIDE);

    bp.routes(from![crate]);
    bp
}
