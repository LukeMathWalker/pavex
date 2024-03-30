use pavex::blueprint::Blueprint;
use pavex::blueprint::router::GET;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(super::a));
    bp.request_scoped(f!(super::b));
    bp.route(GET, "/", f!(super::handler));
    bp
}
