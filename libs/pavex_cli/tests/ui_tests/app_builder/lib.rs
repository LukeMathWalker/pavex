use std::path::PathBuf;

use pavex::blueprint::{
    constructor::{CloningStrategy, Lifecycle},
    router::GET,
    Blueprint,
};
use pavex::response::Response;
use pavex::{f, t};

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

#[derive(Clone)]
pub struct HttpClient;

pub fn http_client(_config: Config) -> HttpClient {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.prebuilt(t!(crate::Config)).clone_if_necessary();
    bp.singleton(f!(crate::http_client)).clone_if_necessary();
    bp.request_scoped(f!(crate::extract_path))
        .error_handler(f!(crate::handle_extract_path_error));
    bp.transient(f!(crate::logger));
    bp.route(GET, "/home", f!(crate::stream_file));
    bp
}
