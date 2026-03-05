use pavex::{blueprint::from, Blueprint};

#[pavex::singleton]
pub fn constructor_with_output_array() -> [u8; 4] {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_input: [u8; 4]) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
