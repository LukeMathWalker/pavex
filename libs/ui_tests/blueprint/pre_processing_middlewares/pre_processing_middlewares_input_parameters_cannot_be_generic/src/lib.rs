use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;
use pavex::middleware::Processing;
use pavex::response::Response;

pub struct GenericType<V>(V);

pub fn generic<T>(_generic_input: GenericType<T>) -> Processing {
    todo!()
}

pub fn doubly_generic<T, S>(_i1: GenericType<T>, _i2: GenericType<S>) -> Processing {
    todo!()
}

pub fn triply_generic<T, S, U>(
    _i1: GenericType<T>,
    _i2: GenericType<S>,
    _i3: GenericType<U>,
) -> Processing {
    todo!()
}

pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.pre_process(f!(crate::generic));
    bp.pre_process(f!(crate::doubly_generic));
    bp.pre_process(f!(crate::triply_generic));
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
