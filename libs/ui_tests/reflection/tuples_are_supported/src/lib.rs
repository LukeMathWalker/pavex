use pavex::{blueprint::from, Blueprint};

#[pavex::singleton]
pub fn constructor_with_output_tuple() -> (usize, isize) {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_input: (usize, isize)) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
