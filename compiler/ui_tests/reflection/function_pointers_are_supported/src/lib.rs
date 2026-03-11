use pavex::{blueprint::from, Blueprint};

#[pavex::request_scoped]
pub fn build_fn_pointer() -> fn(u32) -> u8 {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_f: fn(u32) -> u8) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
