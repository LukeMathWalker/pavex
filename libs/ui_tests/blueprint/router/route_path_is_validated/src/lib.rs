use pavex::blueprint::{from, Blueprint};

#[pavex::get(path = "api")]
pub fn missing_leading_slash() -> String {
    todo!()
}

#[pavex::get(path = "")]
// Empty path is accepted.
pub fn empty_path() -> String {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.routes(from![crate]);
    bp
}
