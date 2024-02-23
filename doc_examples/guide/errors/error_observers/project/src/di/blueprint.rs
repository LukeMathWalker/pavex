use pavex::blueprint::constructor::Lifecycle;
use pavex::blueprint::router::POST;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(super::RootSpan::new), Lifecycle::RequestScoped);
    bp.error_observer(f!(super::error_logger));
    bp.route(POST, "/di", f!(super::handler))
        .error_handler(f!(super::error2response));
    bp
}
