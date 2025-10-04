use pavex::middleware::Next;
use pavex::Response;
use pavex::{blueprint::from, Blueprint};

#[pavex::wrap(id = "EHANDLER_VIA_DEFAULT")]
/// The default error handler is invoked.
pub fn via_attribute<T>(_next: Next<T>) -> Result<Response, CustomError>
where
    T: IntoFuture<Output = Response>,
{
    todo!()
}
#[pavex::wrap(id = "EHANDLER_OVERRIDE_VIA_BLUEPRINT")]
// Error handler is overridden when the middleware is registered against the blueprint.
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

    bp.wrap(EHANDLER_VIA_DEFAULT);
    bp.wrap(EHANDLER_OVERRIDE_VIA_BLUEPRINT)
        .error_handler(CUSTOM_ERROR_INTO_RESPONSE_OVERRIDE);

    bp.routes(from![crate]);
    bp
}
