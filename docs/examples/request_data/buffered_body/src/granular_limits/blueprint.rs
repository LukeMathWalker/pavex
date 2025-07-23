//! px:granular_limits
use super::routes::UPLOAD;
use pavex::Blueprint;
use pavex::request::body::BodySizeLimit;
use pavex::request_scoped;
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
    bp.constructor(UPLOAD_SIZE_LIMIT);
    bp.route(UPLOAD);
    bp
}

#[request_scoped]
pub fn upload_size_limit() -> BodySizeLimit {
    BodySizeLimit::Enabled {
        max_size: 1.gigabytes(),
    }
}
