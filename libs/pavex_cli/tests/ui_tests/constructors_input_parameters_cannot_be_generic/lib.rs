use pavex_builder::{f, router::GET, Blueprint, Lifecycle};

pub struct GenericType<V>(V);

pub fn generic_constructor<T>(generic_input: GenericType<T>) -> u8 {
    todo!()
}

pub fn handler(size: u8) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::generic_constructor), Lifecycle::RequestScoped);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
