use pavex::blueprint::Blueprint;
use pavex::f;
use pavex::request::body::BodySizeLimit;
use pavex::unit::ToByteUnit;

pub fn body_size_limit() -> BodySizeLimit {
    BodySizeLimit::Enabled {
        max_size: 2.megabytes(),
    }
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(self::body_size_limit));
    bp.route(
        pavex::blueprint::router::GET,
        "/custom_limit",
        f!(super::handler),
    );
    bp
}
