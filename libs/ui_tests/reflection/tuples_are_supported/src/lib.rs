use pavex::blueprint::{from, Blueprint};
use pavex::f;

pub fn constructor_with_output_tuple() -> (usize, isize) {
    todo!()
}

#[pavex::get(path = "/home")]
pub fn handler_with_input_tuple(_input: (usize, isize)) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f!(crate::constructor_with_output_tuple));
    bp.routes(from![crate]);
    bp
}
