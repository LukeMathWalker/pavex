use pavex::blueprint::Blueprint;
use pavex::t;

pub struct Unused;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prebuilt(t!(crate::Unused));
    bp
}
