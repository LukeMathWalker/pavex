use pavex_builder::{f, router::GET, Blueprint, Lifecycle};

// Using on purpose a generic parameter that is named differently than the generic parameter
// that appears in the constructor, the `json` function (`T` vs `V`).
pub struct Json<V>(V);

pub fn json<T>() -> Json<T> {
    todo!()
}

pub struct Form<V>(V);

pub fn fallible<T>() -> Result<Form<T>, FallibleError> {
    todo!()
}

pub struct FallibleError;

pub fn error_handler(e: &FallibleError) -> pavex_runtime::response::Response {
    todo!()
}

pub struct FallibleForm<V>(V);

pub fn fallible_with_generic_error<T>() -> Result<FallibleForm<T>, GenericError<T>> {
    todo!()
}

pub fn generic_error_handler<S>(e: &GenericError<S>) -> pavex_runtime::response::Response {
    todo!()
}

pub struct GenericError<P>(P);

pub struct AType;

// The generic parameters of all inputs types are fully specified!
pub fn handler(
    json: Json<u8>,
    json_vec: Json<Vec<u8>>,
    json_ref: &Json<char>,
    fallible: Form<u64>,
    fallible_with_generic_error: FallibleForm<AType>,
) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::json), Lifecycle::RequestScoped);
    bp.constructor(f!(crate::fallible), Lifecycle::RequestScoped)
        .error_handler(f!(crate::error_handler));
    bp.constructor(
        f!(crate::fallible_with_generic_error),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(crate::generic_error_handler));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
