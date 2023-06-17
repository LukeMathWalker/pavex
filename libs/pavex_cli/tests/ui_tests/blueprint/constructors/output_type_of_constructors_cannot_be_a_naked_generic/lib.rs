use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;

pub fn naked<T>() -> T {
    todo!()
}

pub fn fallible_naked<T>() -> Result<T, FallibleError> {
    todo!()
}

pub struct FallibleError;

pub fn error_handler(e: &FallibleError) -> pavex::response::Response {
    todo!()
}

pub fn handler(a: u8, b: u16) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::naked), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::fallible_naked), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
