use pavex::blueprint::{from, Blueprint};

#[pavex::singleton]
pub fn singleton() -> Box<dyn std::error::Error> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp
}
