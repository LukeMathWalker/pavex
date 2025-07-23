use pavex::{blueprint::from, Blueprint};

#[pavex::prebuilt]
#[derive(Clone)]
pub struct Unused;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp
}
