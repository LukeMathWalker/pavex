use pavex::blueprint::router::GET;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.wrap(f!(crate::wrap1));
    bp.nest(nested());
    bp.wrap(f!(crate::wrap2));
    bp
}

pub fn nested() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/", f!(super::handler));
    bp
}
