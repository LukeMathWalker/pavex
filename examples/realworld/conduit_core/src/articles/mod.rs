use pavex_builder::{Blueprint, router::GET, f};
use pavex_runtime::hyper::StatusCode;

pub(crate) fn articles_bp() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.route(GET, "/", f!(crate::articles::get_articles));
    bp
}

pub fn get_articles() -> StatusCode {
    StatusCode::OK
}