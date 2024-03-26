use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;

pub struct GenericType<V>(V);

pub fn generic<T>(_response: Response, generic_input: GenericType<T>) -> Response {
    todo!()
}

pub fn doubly_generic<T, S>(
    _response: Response,
    i1: GenericType<T>,
    i2: GenericType<S>,
) -> Response {
    todo!()
}

pub fn triply_generic<T, S, U>(
    _response: Response,
    i1: GenericType<T>,
    i2: GenericType<S>,
    i3: GenericType<U>,
) -> Response {
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
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
