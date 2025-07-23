use pavex::{blueprint::from, Blueprint};

// Using on purpose a generic parameter that is named differently than the generic parameter
// that appears in the constructor, the `json` function (`T` vs `V`).
pub struct Json<V>(V);

#[pavex::request_scoped]
pub fn json<T>() -> Json<T> {
    todo!()
}

pub struct Form<V>(V);

#[pavex::request_scoped]
pub fn fallible<T>() -> Result<Form<T>, FallibleError> {
    todo!()
}

pub struct FallibleError;

#[pavex::error_handler]
pub fn error_handler(_e: &FallibleError) -> pavex::response::Response {
    todo!()
}

pub struct FallibleForm<V>(V);

// We have a generic parameter `T` in the constructed type **as well as** in the error type.
#[pavex::request_scoped]
pub fn fallible_with_generic_error<T>() -> Result<FallibleForm<T>, GenericError<T>> {
    todo!()
}

#[pavex::error_handler]
pub fn generic_error_handler<S>(_e: &GenericError<S>) -> pavex::response::Response {
    todo!()
}

pub struct FallibleForm2<V>(V);

#[pavex::request_scoped]
pub fn fallible_with_generic_error2<T>() -> Result<FallibleForm2<T>, GenericError2<T>> {
    todo!()
}

// We have the generic parameter `S` in the error type **as well as** in the injected `Json<_>` type.
#[pavex::error_handler]
pub fn doubly_generic_error_handler<S>(
    #[px(error_ref)] _e: &GenericError2<S>,
    _v: &Json<S>,
) -> pavex::response::Response {
    todo!()
}

pub struct GenericError<P>(pub P);
pub struct GenericError2<P>(pub P);

pub struct AType;

// The generic parameters of all inputs types are fully specified!
#[pavex::get(path = "/home")]
pub fn handler(
    _json: Json<u8>,
    _json_vec: Json<Vec<u8>>,
    _json_ref: &Json<char>,
    _fallible: Form<u64>,
    _fallible_with_generic_error: FallibleForm<AType>,
    _fallible_ref_with_generic_error: &FallibleForm<u16>,
    _fallible_ref_with_generic_error2: &FallibleForm2<u8>,
) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
