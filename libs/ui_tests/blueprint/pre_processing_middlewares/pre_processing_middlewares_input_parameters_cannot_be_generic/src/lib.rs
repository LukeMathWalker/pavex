use pavex::blueprint::{from, Blueprint};
use pavex::middleware::Processing;
use pavex::response::Response;

pub struct GenericType<V>(V);

#[pavex::pre_process]
pub fn generic<T>(_generic_input: GenericType<T>) -> Processing {
    todo!()
}

#[pavex::pre_process]
pub fn doubly_generic<T, S>(_i1: GenericType<T>, _i2: GenericType<S>) -> Processing {
    todo!()
}

#[pavex::pre_process]
pub fn triply_generic<T, S, U>(
    _i1: GenericType<T>,
    _i2: GenericType<S>,
    _i3: GenericType<U>,
) -> Processing {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler() -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.pre_process(GENERIC);
    bp.pre_process(DOUBLY_GENERIC);
    bp.pre_process(TRIPLY_GENERIC);
    bp.routes(from![crate]);
    bp
}
