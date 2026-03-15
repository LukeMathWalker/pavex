use std::ptr::NonNull;

use pavex::{blueprint::from, Blueprint};

#[pavex::request_scoped]
pub fn non_null() -> NonNull<u8> {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_n: NonNull<u8>) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
