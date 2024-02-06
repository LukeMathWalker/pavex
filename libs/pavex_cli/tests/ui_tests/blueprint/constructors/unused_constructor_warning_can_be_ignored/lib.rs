use pavex::blueprint::{constructor::Lifecycle, linter::Lint, Blueprint};
use pavex::f;

pub struct Unused;

impl Unused {
    pub fn new() -> Self {
        Self
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::Unused::new), Lifecycle::RequestScoped)
        .ignore(Lint::Unused);
    bp
}
