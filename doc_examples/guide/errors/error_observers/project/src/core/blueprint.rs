use pavex::blueprint::router::POST;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.error_observer(f!(super::error_logger));
    bp.route(POST, "/core", f!(super::handler))
        .error_handler(f!(super::error2response));
    bp
}
