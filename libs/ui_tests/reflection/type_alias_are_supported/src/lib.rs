use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub type MyTupleAlias = (bool, char, u8);
pub type RemoteAlias = dep::IntermediateAlias;
pub type RemoteGenericAlias<T> = dep::IntermediateGenericAlias<T, T>;

pub fn constructor_with_output_tuple() -> (bool, char, u8) {
    todo!()
}

pub fn handler_with_input_tuple(
    _input: MyTupleAlias,
    _a: &RemoteAlias,
    _b: &RemoteGenericAlias<bool>,
) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(crate::constructor_with_output_tuple));
    bp.singleton(f!(crate::RemoteAlias::new));
    bp.singleton(f!(crate::RemoteGenericAlias::<std::primitive::bool>::new));
    bp.route(GET, "/home", f!(crate::handler_with_input_tuple));
    bp
}
