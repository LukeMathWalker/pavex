use pavex::{blueprint::from, Blueprint};

#[pavex::request_scoped]
pub fn const_ptr() -> *const u8 {
    todo!()
}

#[pavex::request_scoped]
pub fn mut_ptr() -> *mut u8 {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_c: *const u8, _m: *mut u8) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
