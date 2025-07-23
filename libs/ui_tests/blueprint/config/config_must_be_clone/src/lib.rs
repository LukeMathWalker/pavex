use pavex::response::Response;
use pavex::{blueprint::from, Blueprint};

// Not cloneable.
#[pavex::config(key = "a", id = "CONFIG_A")]
pub struct A;

// Not cloneable.
// Should error even if marked as never clone.
#[pavex::config(key = "b", id = "CONFIG_B", never_clone)]
pub struct B;

#[pavex::get(path = "/")]
pub fn handler(_a: A, _b: B) -> Response {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.routes(from![crate]);
    bp
}
