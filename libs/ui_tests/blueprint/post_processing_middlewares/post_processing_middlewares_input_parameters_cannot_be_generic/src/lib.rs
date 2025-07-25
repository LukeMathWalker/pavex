use pavex::Response;
use pavex::{blueprint::from, Blueprint};

pub struct GenericType<V>(V);

#[pavex::post_process]
pub fn generic<T>(_response: Response, _generic_input: GenericType<T>) -> Response {
    todo!()
}

#[pavex::post_process]
pub fn doubly_generic<T, S>(
    _response: Response,
    _i1: GenericType<T>,
    _i2: GenericType<S>,
) -> Response {
    todo!()
}

#[pavex::post_process]
pub fn triply_generic<T, S, U>(
    _response: Response,
    _i1: GenericType<T>,
    _i2: GenericType<S>,
    _i3: GenericType<U>,
) -> Response {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler() -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.post_process(GENERIC);
    bp.post_process(DOUBLY_GENERIC);
    bp.post_process(TRIPLY_GENERIC);
    bp.routes(from![crate]);
    bp
}
