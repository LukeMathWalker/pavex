use pavex::blueprint::router::GET;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(f!(crate::core::middleware));
    bp.route(GET, "/", f!(crate::core::handler));
    bp
}
