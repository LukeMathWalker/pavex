use pavex::blueprint::router::GET;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(f!(crate::logging::middleware));
    bp.route(GET, "/logging", f!(crate::logging::handler));
    bp
}
