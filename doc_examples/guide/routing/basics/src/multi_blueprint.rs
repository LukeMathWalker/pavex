use pavex::blueprint::router::{PATCH, POST};
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(POST.or(PATCH), "/article", f!(crate::routes::article));
    bp
}
