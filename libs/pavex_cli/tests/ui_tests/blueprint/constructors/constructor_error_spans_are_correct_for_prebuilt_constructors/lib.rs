use pavex::blueprint::{
    constructor::{Constructor, Lifecycle},
    router::GET,
    Blueprint,
};
use pavex::f;

pub struct GenericType<V>(V);

pub fn generic_constructor<T>(generic_input: GenericType<T>) -> u8 {
    todo!()
}

pub fn handler(i: u8) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    Constructor::new(f!(crate::generic_constructor), Lifecycle::RequestScoped).register(&mut bp);
    bp.route(GET, "/home", f!(crate::handler));
    bp
}
