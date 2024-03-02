use pavex::blueprint::Blueprint;
use pavex::f;
use pavex::request::body::BodySizeLimit;

pub fn body_size_limit() -> BodySizeLimit {
    BodySizeLimit::Disabled
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.request_scoped(f!(self::body_size_limit));
    bp.route(
        pavex::blueprint::router::GET,
        "/no_limit",
        f!(super::handler),
    );
    bp
}
