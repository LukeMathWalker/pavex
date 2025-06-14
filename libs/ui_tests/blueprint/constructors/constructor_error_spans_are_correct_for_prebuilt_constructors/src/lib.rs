use pavex::blueprint::{
    constructor::{Constructor, Lifecycle},
    from,
    Blueprint,
};
use pavex::f;

pub struct GenericType<V>(V);

pub fn generic_constructor<T>(_generic_input: GenericType<T>) -> u8 {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler(_i: u8) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    Constructor::new(f!(crate::generic_constructor), Lifecycle::RequestScoped).register(&mut bp);
    bp.routes(from![crate]);
    bp
}
