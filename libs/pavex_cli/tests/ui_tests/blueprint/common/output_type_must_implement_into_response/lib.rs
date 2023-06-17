use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;

pub fn request_scoped() -> Result<String, ErrorType> {
    todo!()
}

#[derive(Debug)]
pub struct ErrorType;

// It doesn't implement IntoResponse!
pub struct MyCustomOutputType;

pub fn handler(_s: String) -> Result<MyCustomOutputType, ErrorType> {
    todo!()
}

pub fn error_handler(e: &ErrorType) -> MyCustomOutputType {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::request_scoped), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));
    bp.route(GET, "/home", f!(crate::handler))
        .error_handler(f!(crate::error_handler));
    bp
}
