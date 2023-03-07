use pavex_builder::{f, router::GET, Blueprint, Lifecycle};
use pavex_runtime::extract::path::Path;

pub struct HomePath {
    pub home_ids: u32,
}

pub fn request_handler(_path: Path<HomePath>) -> pavex_runtime::response::Response {
    todo!()
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
