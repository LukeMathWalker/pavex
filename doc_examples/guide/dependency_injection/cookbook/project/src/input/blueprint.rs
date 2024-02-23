use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(super::length::<super::GreetBody>));
    bp.request_scoped(f!(super::json));
    bp
}
