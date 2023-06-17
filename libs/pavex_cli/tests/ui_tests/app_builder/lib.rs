use std::path::PathBuf;

use pavex::blueprint::{constructor::Lifecycle, router::GET, Blueprint};
use pavex::f;
use pavex::response::Response;

pub struct Logger;

pub async fn extract_path(
    _inner: pavex::request::RequestHead,
) -> Result<PathBuf, ExtractPathError<String>> {
    todo!()
}

pub struct ExtractPathError<T>(T);

pub fn handle_extract_path_error(_e: &ExtractPathError<String>, _logger: Logger) -> Response {
    todo!()
}

pub fn logger() -> Logger {
    todo!()
}

pub fn stream_file(_inner: PathBuf, _logger: Logger, _http_client: HttpClient) -> Response {
    todo!()
}

#[derive(Clone)]
pub struct Config;

pub fn config() -> Config {
    todo!()
}

#[derive(Clone)]
pub struct HttpClient;

pub fn http_client(_config: Config) -> HttpClient {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.constructor(f!(crate::http_client), Lifecycle::Singleton);
    bp.constructor(f!(crate::extract_path), Lifecycle::RequestScoped)
        .error_handler(f!(crate::handle_extract_path_error));
    bp.constructor(f!(crate::logger), Lifecycle::Transient);
    bp.route(GET, "/home", f!(crate::stream_file));
    bp
}
