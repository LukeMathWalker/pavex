use pavex::blueprint::router::POST;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.error_observer(f!(crate::core::error_logger));
    bp.route(POST, "/core", f!(crate::core::handler))
        .error_handler(f!(crate::core::error2response));
    bp
}
