use pavex::blueprint::router::GET;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(f!(super::timeout))
        .error_handler(f!(super::timeout_error_handler));
    bp.route(GET, "/fallible", f!(super::handler));
    bp
}
