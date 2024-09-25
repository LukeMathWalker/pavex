use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;

pub type MyTupleAlias = (bool, char, u8);
pub type RemoteAlias = dep::IntermediateAlias;

pub fn constructor_with_output_tuple() -> (bool, char, u8) {
    todo!()
}

pub fn handler_with_input_tuple(
    _input: MyTupleAlias,
    _a: &RemoteAlias,
) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(
        f!(crate::constructor_with_output_tuple),
        Lifecycle::Singleton,
    );
    bp.constructor(f!(crate::RemoteAlias::new), Lifecycle::Singleton);
    bp.route(GET, "/home", f!(crate::handler_with_input_tuple));
    bp
}
