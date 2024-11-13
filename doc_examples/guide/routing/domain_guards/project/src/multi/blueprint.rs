use pavex::blueprint::router::GET;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.domain("{user_id}.{tenant_id}.pavex.dev").nest(user_bp());
    bp
}

fn user_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/", f!(super::index));
    // Other routes...
    bp
}
