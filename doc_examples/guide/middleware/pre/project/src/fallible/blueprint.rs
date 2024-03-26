use pavex::blueprint::router::GET;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.pre_process(f!(super::reject_anonymous))
        .error_handler(f!(super::auth_error_handler));
    bp.route(GET, "/", f!(super::handler));
    bp
}
