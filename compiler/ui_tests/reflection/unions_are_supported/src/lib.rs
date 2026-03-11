use pavex::{blueprint::from, Blueprint};

#[repr(C)]
pub union MyUnion {
    pub f: u32,
    pub b: u8,
}

#[pavex::request_scoped]
pub fn build_union() -> MyUnion {
    todo!()
}

#[pavex::get(path = "/")]
pub fn handler(_u: MyUnion) -> pavex::Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
