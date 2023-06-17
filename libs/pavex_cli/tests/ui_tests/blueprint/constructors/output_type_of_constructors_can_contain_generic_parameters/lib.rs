use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;

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

pub fn error_handler(e: &FallibleError) -> pavex::response::Response {
    todo!()
}

pub struct FallibleForm<V>(V);

// We have a generic parameter `T` in the constructed type **as well as** in the error type.
pub fn fallible_with_generic_error<T>() -> Result<FallibleForm<T>, GenericError<T>> {
    todo!()
}

pub fn generic_error_handler<S>(e: &GenericError<S>) -> pavex::response::Response {
    todo!()
}

pub struct FallibleForm2<V>(V);

pub fn fallible_with_generic_error2<T>() -> Result<FallibleForm2<T>, GenericError<T>> {
    todo!()
}

// We have the generic parameter `S` in the error type **as well as** in the injected `Json<_>` type.
pub fn doubly_generic_error_handler<S>(
    e: &GenericError<S>,
    v: &Json<S>,
) -> pavex::response::Response {
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
    fallible_ref_with_generic_error: &FallibleForm<u16>,
    fallible_ref_with_generic_error2: &FallibleForm2<u8>,
) -> pavex::response::Response {
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
    bp.constructor(
        f!(crate::fallible_with_generic_error2),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(crate::doubly_generic_error_handler));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
