use pavex::blueprint::{from, Blueprint};
use pavex::t;

pub struct Unused;

#[pavex::prebuilt]
#[derive(Clone)]
pub struct Unused1;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.prebuilt(t!(crate::Unused));
    bp
}
