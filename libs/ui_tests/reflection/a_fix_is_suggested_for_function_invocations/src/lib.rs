use pavex::blueprint::Blueprint;
use pavex::f;

pub fn my_f() {}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(self::my_f()));
    bp
}
