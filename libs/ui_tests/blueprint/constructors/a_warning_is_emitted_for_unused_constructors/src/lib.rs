use pavex::blueprint::{from, Blueprint};
use pavex::f;

pub struct Unused;

impl Default for Unused {
    fn default() -> Self {
        Self::new()
    }
}

impl Unused {
    pub fn new() -> Self {
        Self
    }
}

pub struct AnnotatedUnused;

#[pavex::request_scoped]
pub fn annotated() -> AnnotatedUnused {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.request_scoped(f!(crate::Unused::new));
    bp
}
