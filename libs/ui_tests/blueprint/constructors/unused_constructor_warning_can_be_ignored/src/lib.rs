use pavex::blueprint::{from, Blueprint};

pub struct Unused;

impl Default for Unused {
    fn default() -> Self {
        Self::new()
    }
}

#[pavex::methods]
impl Unused {
    #[request_scoped(allow(unused))]
    pub fn new() -> Self {
        Self
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp
}
