use pavex::blueprint::Blueprint;
use pavex::f;

pub fn c() -> Box<dyn std::error::Error> {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.singleton(f![crate::c]);
    bp
}
