use pavex::blueprint::router::GET;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.domain("{*any}.example.dev").nest(sub_bp());
    bp
}

fn sub_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/", f!(super::handler));
    // Other routes...
    bp
}
