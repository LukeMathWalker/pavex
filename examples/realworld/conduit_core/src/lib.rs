use articles::articles_bp;
use pavex_builder::{f, router::GET, Blueprint};
use pavex_runtime::hyper::StatusCode;

mod articles;

pub fn ping() -> StatusCode {
    StatusCode::OK
}

pub fn api_blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/api/ping", f!(crate::ping));
    bp.nest_at("/articles", articles_bp());
    bp
}
