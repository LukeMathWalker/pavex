use enumflags2::{bitflags, BitFlags};
use pavex::{blueprint::from, Blueprint};

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MyFlag {
    A = 0b0001,
    B = 0b0010,
}

#[pavex::request_scoped]
pub fn flags() -> BitFlags<MyFlag> {
    BitFlags::empty()
}

#[pavex::get(path = "/")]
pub fn handler(_flags: BitFlags<MyFlag>) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
