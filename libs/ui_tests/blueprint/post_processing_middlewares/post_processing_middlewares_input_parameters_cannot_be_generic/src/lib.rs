use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

pub struct GenericType<V>(V);

pub fn generic<T>(_response: Response, _generic_input: GenericType<T>) -> Response {
    todo!()
}

pub fn doubly_generic<T, S>(
    _response: Response,
    _i1: GenericType<T>,
    _i2: GenericType<S>,
) -> Response {
    todo!()
}

pub fn triply_generic<T, S, U>(
    _response: Response,
    _i1: GenericType<T>,
    _i2: GenericType<S>,
    _i3: GenericType<U>,
) -> Response {
    todo!()
}

#[pavex::post_process]
pub fn generic1<T>(_response: Response, _generic_input: GenericType<T>) -> Response {
    todo!()
}

pub fn handler() -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.post_process(f!(crate::generic));
    bp.post_process(f!(crate::doubly_generic));
    bp.post_process(f!(crate::triply_generic));
    bp.post_process(GENERIC_1);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
