use pavex::blueprint::{constructor::Lifecycle, Blueprint};
use pavex::f;

#[derive(Clone)]
pub struct Unused;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.state_input(f!(crate::Unused));
    bp
}
