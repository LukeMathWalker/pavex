use pavex::blueprint::router::{GET, HEAD};
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET.or(HEAD), "/greet", f!(crate::routes::greet));
    bp
}
