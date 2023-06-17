use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;

pub struct GenericType<V>(V);

pub fn generic_constructor<T>(generic_input: GenericType<T>) -> u8 {
    todo!()
}

pub fn doubly_generic_constructor<T, S>(i1: GenericType<T>, i2: GenericType<S>) -> u16 {
    todo!()
}

pub fn triply_generic_constructor<T, S, U>(
    i1: GenericType<T>,
    i2: GenericType<S>,
    i3: GenericType<U>,
) -> u32 {
    todo!()
}

pub fn handler(i: u8, j: u16, k: u32) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::generic_constructor), Lifecycle::RequestScoped);
    bp.constructor(
        f!(crate::doubly_generic_constructor),
        Lifecycle::RequestScoped,
    );
    bp.constructor(
        f!(crate::triply_generic_constructor),
        Lifecycle::RequestScoped,
    );
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
