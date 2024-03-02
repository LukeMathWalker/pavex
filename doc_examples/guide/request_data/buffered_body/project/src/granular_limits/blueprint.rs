use pavex::blueprint::{router::POST, Blueprint};
use pavex::f;
use pavex::request::body::BodySizeLimit;
use pavex::unit::ToByteUnit;

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.nest(upload_bp());
    // Other routes...
    bp
}

fn upload_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    // This limit will only apply to the routes registered
    // in this nested blueprint.
    bp.request_scoped(f!(self::upload_size_limit));
    bp.route(POST, "/upload", f!(crate::upload));
    bp
}

pub fn upload_size_limit() -> BodySizeLimit {
    BodySizeLimit::Enabled {
        max_size: 1.gigabytes(),
    }
}
