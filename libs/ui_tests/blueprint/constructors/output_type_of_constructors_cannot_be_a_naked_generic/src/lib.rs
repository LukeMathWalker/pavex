use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub fn naked<T>() -> T {
    todo!()
}

pub fn fallible_naked<T>() -> Result<T, FallibleError> {
    todo!()
}

pub struct FallibleError;

pub fn error_handler(_e: &FallibleError) -> pavex::response::Response {
    todo!()
}

pub fn handler(_a: u8, _b: u16) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::naked));
    bp.request_scoped(f!(crate::fallible_naked))
        .error_handler(f!(crate::error_handler));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
