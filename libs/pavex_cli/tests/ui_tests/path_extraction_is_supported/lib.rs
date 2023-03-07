use pavex_builder::{f, router::GET, Blueprint, Lifecycle};
use pavex_runtime::extract::path::Path;

pub struct HomePath {
    pub home_id: u32,
}

pub fn request_handler(path: Path<HomePath>) -> String {
    format!("{}", path.0.home_id)
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(
        f!(pavex_runtime::extract::path::Path::extract),
        Lifecycle::RequestScoped,
    )
    .error_handler(f!(
        pavex_runtime::extract::path::ExtractPathError::default_error_handler
    ));
    bp.route(GET, "/home/:home_id", f!(crate::request_handler));
    bp
}
