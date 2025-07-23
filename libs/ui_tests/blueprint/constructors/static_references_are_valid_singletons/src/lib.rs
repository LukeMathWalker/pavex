use pavex::{blueprint::from, Blueprint};

#[pavex::singleton]
pub fn static_str() -> &'static str {
    todo!()
}

#[pavex::singleton]
pub fn static_u8() -> &'static u8 {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_x: &'static str, _y: &'static u8) -> pavex::response::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
