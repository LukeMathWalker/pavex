use pavex::blueprint::{constructor::Lifecycle, Blueprint};
use pavex::f;

pub struct Unused;

impl Unused {
    pub fn new() -> Self {
        Self
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::Unused::new));
    bp
}
