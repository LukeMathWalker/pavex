use std::path::PathBuf;

use pavex::blueprint::{from, Blueprint};
use pavex::response::Response;
use pavex::f;

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

#[pavex::get(path = "/home")]
pub fn stream_file(_inner: PathBuf, _logger: Logger, _http_client: HttpClient) -> Response {
    todo!()
}

#[derive(Clone)]
#[pavex::prebuilt(clone_if_necessary)]
pub struct Config;

#[derive(Clone)]
pub struct HttpClient;

pub fn http_client(_config: Config) -> HttpClient {
    todo!()
}

pub fn blueprint() -> Blueprint {
    let mut bp = Blueprint::new();
    bp.import(from![crate]);
    bp.singleton(f!(crate::http_client)).clone_if_necessary();
    bp.request_scoped(f!(crate::extract_path))
        .error_handler(f!(crate::handle_extract_path_error));
    bp.transient(f!(crate::logger));
    bp.routes(from![crate]);
    bp
}
