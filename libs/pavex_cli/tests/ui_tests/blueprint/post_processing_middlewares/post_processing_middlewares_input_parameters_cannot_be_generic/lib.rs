use std::future::IntoFuture;

use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Next;
use pavex::response::Response;

pub struct GenericType<V>(V);

pub fn generic_wrapping_middleware<T>(
    _response: Response,
    generic_input: GenericType<T>,
) -> Response {
    todo!()
}

pub fn doubly_generic_wrapping_middleware<T, S>(
    _response: Response,
    i1: GenericType<T>,
    i2: GenericType<S>,
) -> Response {
    todo!()
}

pub fn triply_generic_wrapping_middleware<T, S, U>(
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
    bp.post_process(f!(crate::generic_wrapping_middleware));
    bp.post_process(f!(crate::doubly_generic_wrapping_middleware));
    bp.post_process(f!(crate::triply_generic_wrapping_middleware));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
