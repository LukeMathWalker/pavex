use pavex::blueprint::constructor::Lifecycle;
use pavex::blueprint::router::POST;
use pavex::blueprint::Blueprint;
use pavex::f;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::di::RootSpan::new), Lifecycle::RequestScoped);
    bp.error_observer(f!(crate::core::error_logger));
    bp.route(POST, "/di", f!(crate::core::handler))
        .error_handler(f!(crate::core::error2response));
    bp
}
