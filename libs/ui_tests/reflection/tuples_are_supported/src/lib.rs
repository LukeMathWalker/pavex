use pavex::blueprint::{router::GET, Blueprint};
use pavex::f;

pub fn constructor_with_output_tuple() -> (usize, isize) {
    todo!()
}

pub fn handler_with_input_tuple(_input: (usize, isize)) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(crate::constructor_with_output_tuple));
    bp.route(GET, "/home", f!(crate::handler_with_input_tuple));
    bp
}
