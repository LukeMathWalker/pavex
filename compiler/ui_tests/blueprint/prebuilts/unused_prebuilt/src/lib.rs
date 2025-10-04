use pavex::{blueprint::from, Blueprint};

#[pavex::prebuilt]
#[derive(Clone)]
// We should get a warning for this prebuilt...
pub struct Unused;

#[pavex::prebuilt(allow(unused))]
#[derive(Clone)]
// ...but no warning for this one.
pub struct AllowedUnused;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp
}
