use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;

pub type MyTupleAlias = (bool, char, u8);

pub fn constructor_with_output_tuple() -> (bool, char, u8) {
    todo!()
}

pub fn handler_with_input_tuple(input: MyTupleAlias) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(
        f!(crate::constructor_with_output_tuple),
        Lifecycle::Singleton,
    );
    bp.route(GET, "/home", f!(crate::handler_with_input_tuple));
    bp
}
