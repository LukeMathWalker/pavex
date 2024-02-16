use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

pub fn constructor() -> String {
    todo!()
}

pub fn error_handler(e: &pavex::Error, s: &mut String) -> Response {
    todo!()
}

pub fn handler() -> Result<Response, pavex::Error> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::constructor), Lifecycle::RequestScoped);
    bp.route(GET, "/home", f!(crate::handler))
        .error_handler(f!(crate::error_handler));
    bp
}
