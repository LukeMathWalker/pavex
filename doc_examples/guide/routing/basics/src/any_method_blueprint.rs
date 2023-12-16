use pavex::blueprint::router::ANY;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(ANY, "/article", f!(crate::routes::article));
    bp
}
