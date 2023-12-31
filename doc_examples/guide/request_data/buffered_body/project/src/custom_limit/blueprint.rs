use pavex::blueprint::{constructor::Lifecycle, Blueprint};
use pavex::f;
use pavex::request::body::BodySizeLimit;

pub fn body_size_limit() -> BodySizeLimit {
    BodySizeLimit::Enabled {
        max_n_bytes: 10_485_760, // 10 MBs
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::body_size_limit), Lifecycle::Singleton);
    bp.route(
        pavex::blueprint::router::GET,
        "/custom_limit",
        f!(crate::custom_limit::handler),
    );
    bp
}
