use pavex_builder::{f, Blueprint, Lifecycle};

pub type MyTupleAlias = (bool, char, u8);

pub fn constructor_with_output_tuple() -> (bool, char, u8) {
    todo!()
}

pub fn handler_with_input_tuple(input: MyTupleAlias) -> pavex_runtime::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(
        f!(crate::constructor_with_output_tuple),
        Lifecycle::Singleton,
    );
    bp.route(f!(crate::handler_with_input_tuple), "/home");
    bp
}
