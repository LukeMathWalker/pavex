use pavex_builder::{f, router::GET, Blueprint, Lifecycle};

pub fn json<T>() -> Json<T> {
    todo!()
}

pub struct Json<T>(T);

pub fn fallible<T>() -> Result<Form<T>, FallibleError> {
    todo!()
}

pub struct FallibleError;

pub fn error_handler(e: &FallibleError) -> pavex_runtime::response::Response {
    todo!()
}

pub struct Form<T>(T);

// The generic parameters of all inputs types are fully specified!
pub fn handler(
    json: Json<u8>,
    json_vec: Json<Vec<u8>>,
    json_ref: &Json<char>,
    fallible: Form<u64>,
) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::json), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::fallible), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
