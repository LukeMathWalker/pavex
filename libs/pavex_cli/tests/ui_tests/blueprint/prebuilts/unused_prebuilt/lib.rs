use pavex::blueprint::{constructor::Lifecycle, Blueprint};
use pavex::t;

pub struct Unused;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prebuilt(t!(crate::Unused));
    bp
}
