use pavex::blueprint::router::POST;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(POST, "/login", f!(crate::core::handler))
        .error_handler(f!(crate::core::login_error2response)); // (1)!
    bp
}
