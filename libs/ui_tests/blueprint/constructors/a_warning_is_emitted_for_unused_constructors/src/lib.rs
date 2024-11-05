use pavex::blueprint::Blueprint;
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

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(crate::Unused::new));
    bp
}
