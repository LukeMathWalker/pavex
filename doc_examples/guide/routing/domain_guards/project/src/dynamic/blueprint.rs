use pavex::blueprint::router::GET;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.domain("{sub}.pavex.dev").nest(sub_bp());
    bp
}

fn sub_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/", f!(super::index));
    // Other routes...
    bp
}
