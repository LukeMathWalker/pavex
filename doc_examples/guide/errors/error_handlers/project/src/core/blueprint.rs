use pavex::blueprint::router::POST;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(POST, "/login", f!(super::handler))
        .error_handler(f!(super::login_error2response)); // (1)!
    bp
}
