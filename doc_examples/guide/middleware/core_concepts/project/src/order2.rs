use pavex::blueprint::router::GET;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(f!(crate::first));
    bp.route(GET, "/", f!(crate::handler));
    bp.wrap(f!(crate::second));
    bp
}
